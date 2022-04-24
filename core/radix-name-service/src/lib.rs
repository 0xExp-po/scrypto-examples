use scrypto::prelude::*;
use sha2::{Digest, Sha256};

#[derive(NonFungibleData)]
struct DomainName {
    #[scrypto(mutable)]
    address: ComponentAddress,
    
    #[scrypto(mutable)]
    last_valid_epoch: u64,
    
    #[scrypto(mutable)]
    deposit_amount: Decimal,
}

// Assuming an average epoch duration of 35 minutes, 15k epochs roughly fit into one year
// This is a very rough estimate, of course
const EPOCHS_PER_YEAR: u64 = 15_000;

blueprint! {

    struct RadixNameService {
        admin_badge: ResourceAddress,
        minter: Vault,
        name_resource: ResourceAddress,
        deposits: Vault,
        fees: Vault,
        deposit_per_year: Decimal,
        fee_address_update: Decimal,
        fee_renewal_per_year: Decimal,
    }

    impl RadixNameService {
        /// Creates a new RNS instance
        pub fn instantiate_rns(
            deposit_per_year: Decimal,
            fee_address_update: Decimal,
            fee_renewal_per_year: Decimal,
        ) -> (ComponentAddress, Bucket) {
            let admin_badge = ResourceBuilder::new_fungible()
                .divisibility(DIVISIBILITY_NONE)
                .initial_supply(dec!("1"));

            let minter = ResourceBuilder::new_fungible()
                .divisibility(DIVISIBILITY_NONE)
                .initial_supply(dec!("1"));

            let name_resource = ResourceBuilder::new_non_fungible()
                .metadata("name", "DomainName")
                .mintable(rule!(require(minter.resource_address())), LOCKED)
                .burnable(rule!(require(minter.resource_address())), LOCKED)
                .updateable_non_fungible_data(rule!(require(minter.resource_address())), LOCKED)
                .no_initial_supply();

            let rules = AccessRules::new()
                .method("burn_expired_names", rule!(require(admin_badge.resource_address())))
                .method("withdraw_fees", rule!(require(admin_badge.resource_address())))
                .default(rule!(allow_all));

            let component = RadixNameService {
                admin_badge: admin_badge.resource_address(),
                minter: Vault::with_bucket(minter),
                name_resource,
                deposits: Vault::new(RADIX_TOKEN),
                fees: Vault::new(RADIX_TOKEN),
                deposit_per_year,
                fee_address_update,
                fee_renewal_per_year,
            }
            .instantiate()
            .add_access_check(rules)
            .globalize();

            (component, admin_badge)
        }

        /// Lookup the address for a given `name`.
        /// Panics if that name is not registered.
        pub fn lookup_address(&self, name: String) -> ComponentAddress {
            let hash = Self::hash_name(name);
            
            let resource_manager = borrow_resource_manager!(self.name_resource);
            let name_data: DomainName = resource_manager.get_non_fungible_data(
                &NonFungibleId::from_bytes(hash.to_be_bytes().to_vec())
            );
            
            name_data.address
        }

        /// Registers the given `name` and maps it to the given `target_address` for `reserve_years`.
        /// The supplied `deposit` is locked until the name is unregistered.
        ///
        /// This method returns an NFT that represents ownership of the registered name and any
        /// overpaid deposit.
        pub fn register_name(
            &mut self,
            name: String,
            target_address: ComponentAddress,
            reserve_years: u8,
            mut deposit: Bucket,
        ) -> (Bucket, Bucket) {
            assert!(name.ends_with(".xrd"), "The domain name must end on '.xrd'");
            assert!(
                reserve_years > 0,
                "A name must be reserved for at least one year"
            );
            assert!(
                deposit.resource_address() == RADIX_TOKEN,
                "The deposit must be made in XRD"
            );

            let hash = Self::hash_name(name);
            let deposit_amount = self.deposit_per_year * Decimal::from(reserve_years);
            let last_valid_epoch =
                Runtime::current_epoch() + EPOCHS_PER_YEAR * u64::from(reserve_years);

            assert!(
                deposit.amount() >= deposit_amount,
                "Insufficient deposit. You need to send a deposit of {} XRD",
                deposit_amount
            );

            let name_data = DomainName {
                address: target_address,
                last_valid_epoch,
                deposit_amount,
            };

            let name_nft = self.minter.authorize(|| {
                let resource_manager = borrow_resource_manager!(self.name_resource);
                resource_manager.mint_non_fungible(
                    &NonFungibleId::from_bytes(hash.to_be_bytes().to_vec()),
                    name_data
                )
            });

            self.deposits.put(deposit.take(deposit_amount));

            (name_nft, deposit)
        }

        /// Unregister the name(s) that is/are represented by the given `name_nft` bucket.
        /// Returns a bucket with the tokens that were initially deposited when the name(s) was/were
        /// registered.
        /// The supplied `name_nft` is burned.
        pub fn unregister_name(&mut self, name_nft: Bucket) -> Bucket {
            assert!(
                name_nft.resource_address() == self.name_resource,
                "The supplied bucket does not contain a domain name NFT"
            );
            assert!(!name_nft.is_empty(), "The supplied bucket is empty");

            let mut total_deposit_amount = Decimal::zero();
            for nft in name_nft.non_fungibles::<DomainName>() {
                total_deposit_amount += nft.data().deposit_amount;
            }

            self.minter.authorize(|| {
                name_nft.burn()
            });

            self.deposits.take(total_deposit_amount)
        }

        /// Updates the address for the name that is represented by the given `name_nft`.
        /// The fee is not added to the initial deposit and is not returned when the name is
        /// unregistered.
        /// Returns any overpaid fees.
        pub fn update_address(
            &mut self,
            name_nft: Proof,
            new_address: ComponentAddress,
            mut fee: Bucket,
        ) -> Bucket {
            assert!(
                name_nft.resource_address() == self.name_resource,
                "The name_nft bucket does not contain a domain name NFT"
            );
            assert!(
                name_nft.amount() == Decimal::one(),
                "The name_nft bucket must contain exactly one DomainName NFT"
            );
            assert!(
                fee.resource_address() == RADIX_TOKEN,
                "The fee must be payed in XRD"
            );

            let fee_amount = self.fee_address_update;
            assert!(
                fee.amount() >= fee_amount,
                "Insufficient fee amount. You need to send a fee of {} XRD",
                fee_amount
            );

            let resource_manager: &ResourceManager = borrow_resource_manager!(self.name_resource);

            let id = name_nft.non_fungible::<DomainName>().id();
            let old_name_data = resource_manager.get_non_fungible_data::<DomainName>(&id);
            let new_name_data = DomainName {
                address: new_address,
                last_valid_epoch: old_name_data.last_valid_epoch,
                deposit_amount: old_name_data.deposit_amount,
            };

            self.minter.authorize(|| {
                resource_manager.update_non_fungible_data(&id, new_name_data)
            });
            self.fees.put(fee.take(fee_amount));

            name_nft.drop();
            fee
        }

        /// Renews the name identified by the given `name_nft` for `renew_years`.
        /// The fee is not added to the initial deposit and is not returned when the name is
        /// unregistered.
        /// Returns any overpaid fees.
        pub fn renew_name(&mut self, name_nft: Proof, renew_years: u8, mut fee: Bucket) -> Bucket {
            assert!(
                name_nft.resource_address() == self.name_resource,
                "The supplied bucket does not contain a domain name NFT"
            );
            assert!(
                name_nft.amount() == Decimal::one(),
                "The supplied bucket must contain exactly one DomainName NFT"
            );
            assert!(
                fee.resource_address() == RADIX_TOKEN,
                "The fee must be payed in XRD"
            );
            assert!(
                renew_years > 0,
                "The name must be renewed for at least one year"
            );

            let fee_amount = self.fee_renewal_per_year * renew_years;
            assert!(
                fee.amount() >= fee_amount,
                "Insufficient fee amount. You need to send a fee of {} XRD",
                fee_amount
            );

            let resource_manager: &ResourceManager = borrow_resource_manager!(self.name_resource);

            let id = name_nft.non_fungible::<DomainName>().id();
            let mut name_data = resource_manager.get_non_fungible_data::<DomainName>(&id);
            name_data.last_valid_epoch =
                name_data.last_valid_epoch + EPOCHS_PER_YEAR * u64::from(renew_years);
            
            self.minter.authorize(|| {
                resource_manager.update_non_fungible_data(&id, name_data)
            });
            self.fees.put(fee.take(fee_amount));

            name_nft.drop();
            fee
        }

        /// Burns all names that have expired. Must be called regularly.
        pub fn burn_expired_names(&self) {
            todo!("This can be implemented as soon as resources can be recalled from vaults")
        }

        /// Withdraws all fees that have been paid to this component. This does not
        /// include deposits that will be refunded to users upon unregistering their domain names.
        pub fn withdraw_fees(&mut self) -> Bucket {
            self.fees.take_all()
        }

        /// Calculates a hash for the given `name`.
        ///
        /// The hash is calculated by applying SHA256 to the given name
        /// and then taking the output's leftmost bytes to construct a u128
        /// value which can be used as a Scrypto NFT ID.
        fn hash_name(name: String) -> u128 {
            let mut hasher = Sha256::new();
            hasher.update(name);
            let hash = hasher.finalize();
            let mut truncated_hash: [u8; 16] = Default::default();
            truncated_hash.copy_from_slice(&hash[..16]);
            u128::from_le_bytes(truncated_hash)
        }
    }
}
