use scrypto::prelude::*;

#[blueprint]
mod regulated_token {
    enable_method_auth!{
        roles {
            super_admin,
            general_admin
        },
        methods {
            toggle_transfer_freeze => super_admin;
            collect_payments => general_admin;
            advance_stage => general_admin;
            get_current_stage => PUBLIC;
            buy_token => PUBLIC;
        }
    }
    struct RegulatedToken {
        token_supply: Vault,
        internal_authority: Vault,
        collected_xrd: Vault,
        current_stage: u8,
        admin_badge_resource_address: ResourceAddress,
        freeze_badge_resource_address: ResourceAddress,
    }

    impl RegulatedToken {
        pub fn instantiate_regulated_token() -> (Global<RegulatedToken>, Bucket, Bucket) {
            // We will start by creating two tokens we will use as badges and return to our instantiator
            let general_admin: Bucket = ResourceBuilder::new_fungible()
                .divisibility(DIVISIBILITY_NONE)
                .metadata("name", "RegulatedToken general admin badge")
                .burnable(rule!(allow_all), LOCKED)
                .mint_initial_supply(1);

            let freeze_admin: Bucket = ResourceBuilder::new_fungible()
                .divisibility(DIVISIBILITY_NONE)
                .metadata("name", "RegulatedToken freeze-only badge")
                .burnable(rule!(allow_all), LOCKED)
                .mint_initial_supply(1);

            // Next we will create a badge we'll hang on to for minting & transfer authority
            let internal_admin: Bucket = ResourceBuilder::new_fungible()
                .divisibility(DIVISIBILITY_NONE)
                .metadata("name", "RegulatedToken internal authority badge")
                .burnable(rule!(allow_all), LOCKED)
                .mint_initial_supply(1);

            // Next we will create our regulated token with an initial fixed supply of 100 and the appropriate permissions
            let access_rule: AccessRule = rule!(
                require(general_admin.resource_address())
                    || require(internal_admin.resource_address())
            );
            let my_bucket: Bucket = ResourceBuilder::new_fungible()
                .divisibility(DIVISIBILITY_MAXIMUM)
                .metadata("name", "Regulo")
                .metadata("symbol", "REG")
                .metadata(
                    "stage",
                    "Stage 1 - Fixed supply, may be restricted transfer",
                )
                .updateable_metadata(access_rule.clone(), access_rule.clone())
                .restrict_withdraw(access_rule.clone(), access_rule.clone())
                .mintable(access_rule.clone(), access_rule.clone())
                .mint_initial_supply(100);

            // Next we need to setup the access rules for the methods of the component
            // let access_rules_config = AccessRulesConfig::new()
            //     .method(
            //         "toggle_transfer_freeze",
            //         rule!(
            //             require(general_admin.resource_address())
            //                 || require(freeze_admin.resource_address())
            //         ),
            //         AccessRule::DenyAll,
            //     )
            //     .method(
            //         "collect_payments",
            //         rule!(require(general_admin.resource_address())),
            //         AccessRule::DenyAll,
            //     )
            //     .method(
            //         "advance_stage",
            //         rule!(require(general_admin.resource_address())),
            //         AccessRule::DenyAll,
            //     )
            //     .default(rule!(allow_all), AccessRule::DenyAll);


            let component = Self {
                token_supply: Vault::with_bucket(my_bucket),
                internal_authority: Vault::with_bucket(internal_admin),
                collected_xrd: Vault::new(RADIX_TOKEN),
                current_stage: 1,
                admin_badge_resource_address: general_admin.resource_address(),
                freeze_badge_resource_address: freeze_admin.resource_address(),
            }
            .instantiate()
            .prepare_to_globalize(OwnerRole::None)
            .roles(
                roles!(
                    super_admin => rule!(
                        require(general_admin.resource_address())
                            ||  require(freeze_admin.resource_address())
                    ),
                    mutable_by: general_admin;

                    general_admin => rule!(require(general_admin.resource_address())), 
                    mutable_by: general_admin;
                )
            )
            .globalize();

            (
                component,
                general_admin,
                freeze_admin,
            )
        }

        /// Either the general admin or freeze admin badge may be used to freeze or unfreeze consumer transfers of the supply
        pub fn toggle_transfer_freeze(&self, set_frozen: bool) {
            // Note that this operation will fail if the token has reached stage 3 and the token behavior has been locked
            let token_resource_manager = 
                self.token_supply.resource_manager();

            self.internal_authority.authorize(|| {
                if set_frozen {
                    token_resource_manager.set_withdrawable(rule!(
                        require(self.admin_badge_resource_address)
                            || require(self.internal_authority.resource_address())
                    ));
                    info!("Token transfer is now RESTRICTED");
                } else {
                    token_resource_manager.set_withdrawable(rule!(allow_all));
                    info!("Token is now freely transferrable");
                }
            })
        }

        pub fn get_current_stage(&self) -> u8 {
            info!("Current stage is {}", self.current_stage);
            self.current_stage
        }

        /// Permit the proper authority to withdraw our collected XRD
        pub fn collect_payments(&mut self) -> Bucket {
            self.collected_xrd.take_all()
        }

        pub fn advance_stage(&mut self) {
            // Adding the internal admin badge to the component auth zone to allow for the operations below
            LocalAuthZone::push(self.internal_authority.create_proof());

            assert!(self.current_stage <= 2, "Already at final stage");
            let token_resource_manager =
                self.token_supply.resource_manager();

            if self.current_stage == 1 {
                // Advance to stage 2
                // Token will still be restricted transfer upon admin demand, but we will mint beyond the initial supply as required
                self.current_stage = 2;

                // Update token's metadata to reflect the current stage
                token_resource_manager
                    .metadata()
                    .set(
                        "stage",
                        "Stage 2 - Unlimited supply, may be restricted transfer".to_string(),
                    );

                // Enable minting for the token
                token_resource_manager
                    .set_mintable(rule!(require(self.internal_authority.resource_address())));
                info!("Advanced to stage 2");

                // Drop the last added proof to the component auth zone which is the internal admin badge
                LocalAuthZone::pop().drop();
            } else {
                // Advance to stage 3
                // Token will no longer be regulated
                // Restricted transfer will be permanently turned off, supply will be made permanently immutable
                self.current_stage = 3;

                // Update token's metadata to reflect the final stage
                token_resource_manager
                    .metadata()
                    .set(
                        "stage",
                        "Stage 3 - Unregulated token, fixed supply".to_string(),
                    );

                // Set our behavior appropriately now that the regulated period has ended
                token_resource_manager.set_mintable(rule!(deny_all));
                token_resource_manager.set_withdrawable(rule!(allow_all));
                token_resource_manager.set_updateable_metadata(rule!(deny_all));

                // Permanently prevent the behavior of the token from changing
                token_resource_manager.lock_mintable();
                token_resource_manager.lock_withdrawable();
                token_resource_manager.lock_updateable_metadata();

                // With the resource behavior forever locked, our internal authority badge no longer has any use
                // We will burn our internal badge, and the holders of the other badges may burn them at will
                // Our badge has the allows everybody to burn, so there's no need to provide a burning authority

                // Drop the last added proof to the component auth zone which is the internal admin badge
                LocalAuthZone::pop().drop();

                self.internal_authority.take_all().burn();

                info!("Advanced to stage 3");
            }
        }

        /// Buy a quantity of tokens, if the supply on-hand is sufficient, or if current rules permit minting additional supply.
        /// The system will *always* allow buyers to purchase available tokens, even when the token transfers are otherwise frozen
        pub fn buy_token(&mut self, quantity: Decimal, mut payment: Bucket) -> (Bucket, Bucket) {
            assert!(
                quantity > dec!("0"),
                "Can't sell you nothing or less than nothing"
            );
            // Adding the internal admin badge to the component auth zone to allow for the operations below
            LocalAuthZone::push(self.internal_authority.create_proof());

            // Early birds who buy during stage 1 get a discounted rate
            let price: Decimal = if self.current_stage == 1 {
                dec!("50")
            } else {
                dec!("100")
            };

            // Take what we're owed
            self.collected_xrd.put(payment.take(price * quantity));

            // Can we fill the desired quantity from current supply?
            let extra_demand = quantity - self.token_supply.amount();
            if extra_demand <= dec!("0") {
                // Take the required quantity, and return it along with any change
                // The token may currently be under restricted transfer, so we will authorize our withdrawal
                let tokens = self.token_supply.take(quantity);

                // Drop the last added proof to the component auth zone which is the internal admin badge
                LocalAuthZone::pop().drop();

                return (tokens, payment);
            } else {
                // We will attempt to mint the shortfall
                // If we are in stage 1 or 3, this action will fail, and it would probably be a good idea to tell the user this
                // For the purposes of example, we will blindly attempt to mint
                let mut tokens = self.token_supply.resource_manager()
                    .mint(extra_demand);

                // Combine the new tokens with whatever was left in supply to meet the full quantity
                let existing_tokens = self.token_supply.take_all();
                tokens.put(existing_tokens);

                // Drop the last added proof to the component auth zone which is the internal admin badge
                LocalAuthZone::pop().drop();

                // Return the tokens, along with any change
                return (tokens, payment);
            }
        }
    }
}
