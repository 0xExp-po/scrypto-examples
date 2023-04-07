use scrypto::prelude::*;
use std::ops::Deref;

// This is a barebone implementation of Lending protocol.
//
// Following features are missing:
// * Fees
// * Multi-collateral with price oracle
// * Variable interest rate
// * Authorization
// * Interest dynamic adjustment strategy
// * Upgradability

#[blueprint]
mod auto_lend {
    struct AutoLend {
        /// The liquidity pool
        liquidity_pool: Vault,
        /// The min collateral ratio that a user has to maintain
        min_collateral_ratio: Decimal,
        /// The max percent of liquidity pool one can borrow
        max_borrow_percent: Decimal,
        /// The max percent of debt one can liquidate
        max_liquidation_percent: Decimal,
        /// Liquidation bonus
        liquidation_bonus: Decimal,
        /// User state
        users: KeyValueStore<ResourceAddress, User>,
        /// The interest rate of deposits, per epoch
        deposit_interest_rate: Decimal,
        /// The (stable) interest rate of loans, per epoch
        borrow_interest_rate: Decimal,
    }

    impl AutoLend {
        /// Creates a lending pool, with single collateral.
        pub fn instantiate_autolend(reserve_address: ResourceAddress) -> ComponentAddress {
            Self {
                liquidity_pool: Vault::new(reserve_address),
                min_collateral_ratio: dec!("1.2"),
                max_borrow_percent: dec!("0.3"),
                max_liquidation_percent: dec!("0.5"),
                liquidation_bonus: dec!("0.05"),
                users: KeyValueStore::new(),
                deposit_interest_rate: dec!("0.01"),
                borrow_interest_rate: dec!("0.02"),
            }
            .instantiate()
            .globalize()
        }

        /// Registers a new user
        pub fn new_user(&self) -> Bucket {
            ResourceBuilder::new_fungible()
                .divisibility(DIVISIBILITY_NONE)
                .metadata("name", "AutoLend User Badge")
                .mint_initial_supply(1)
        }

        /// Deposits into the liquidity pool and start earning interest.
        pub fn deposit(&mut self, user_auth: Proof, reserve_tokens: Bucket) {
            let user_id = Self::get_user_id(user_auth);
            let amount = reserve_tokens.amount();

            // Update user state
            let deposit_interest_rate = self.deposit_interest_rate;
            let user: User = match self.users.get_mut(&user_id) {
                Some(mut user) => {
                    user.on_deposit(amount, deposit_interest_rate);
                    (*user.deref()).clone()
                }
                None => User {
                    deposit_balance: amount,
                    borrow_balance: Decimal::zero(),
                    deposit_interest_rate,
                    borrow_interest_rate: Decimal::zero(),
                    deposit_last_update: Runtime::current_epoch(),
                    borrow_last_update: Runtime::current_epoch(),
                },
            };

            // Commit state changes
            self.users.insert(user_id, user);
            self.liquidity_pool.put(reserve_tokens);
        }

        /// Redeems the underlying assets, partially or in full.
        pub fn redeem(&mut self, user_auth: Proof, amount: Decimal) -> Bucket {
            let user_id = Self::get_user_id(user_auth);

            // Update user state
            let mut user = self.get_user(user_id);
            let to_return_amount = user.on_redeem(amount);
            user.check_collateral_ratio(self.min_collateral_ratio);

            debug!(
                "LP balance: {}, redeemded: {}",
                self.liquidity_pool.amount(),
                to_return_amount
            );

            // Commit state changes
            self.users.insert(user_id, user);
            self.liquidity_pool.take(to_return_amount)
        }

        /// Borrows the specified amount from lending pool
        pub fn borrow(&mut self, user_auth: Proof, requested: Decimal) -> Bucket {
            let user_id = Self::get_user_id(user_auth);

            assert!(
                requested <= self.liquidity_pool.amount() * self.max_borrow_percent,
                "Max borrow percent exceeded"
            );

            // Update user state
            let borrow_interest_rate = self.borrow_interest_rate;
            let mut user = self.get_user(user_id);
            user.on_borrow(requested, borrow_interest_rate);
            user.check_collateral_ratio(self.min_collateral_ratio);

            // Commit state changes
            self.users.insert(user_id, user);
            self.liquidity_pool.take(requested)
        }

        /// Repays a loan, partially or in full.
        pub fn repay(&mut self, user_auth: Proof, mut repaid: Bucket) -> Bucket {
            let user_id = Self::get_user_id(user_auth);

            // Update user state
            let mut user = self.get_user(user_id);
            let to_return_amount = user.on_repay(repaid.amount());
            let to_return = repaid.take(to_return_amount);

            // Commit state changes
            self.users.insert(user_id, user);
            self.liquidity_pool.put(repaid);
            to_return
        }

        /// Liquidates one user's position, if it's under collateralized.
        pub fn liquidate(&mut self, user_id: ResourceAddress, repaid: Bucket) -> Bucket {
            let mut user = self.get_user(user_id);

            // Check if the user is under collateralized
            let collateral_ratio = user.get_collateral_ratio();
            if let Some(ratio) = collateral_ratio {
                assert!(
                    ratio <= self.min_collateral_ratio,
                    "Liquidation not allowed."
                );
            } else {
                panic!("No borrow from the user");
            }

            // Check liquidation size
            assert!(
                repaid.amount() <= user.borrow_balance * self.max_liquidation_percent,
                "Max liquidation percent exceeded."
            );

            // Update user state
            let to_return_amount = user.on_liquidate(repaid.amount(), self.max_liquidation_percent);
            let to_return = self.liquidity_pool.take(to_return_amount);

            // Commit state changes
            self.users.insert(user_id, user);
            to_return
        }

        /// Displays the current state of a user.
        pub fn get_user(&self, user_id: ResourceAddress) -> User {
            let user = self.users.get(&user_id).expect("User not found");
            (*user.deref()).clone()
        }

        /// Returns the deposit interest rate per epoch
        pub fn set_deposit_interest_rate(&mut self, rate: Decimal) {
            self.deposit_interest_rate = rate;
        }

        /// Returns the borrow interest rate per epoch
        pub fn set_borrow_interest_rate(&mut self, rate: Decimal) {
            self.borrow_interest_rate = rate;
        }

        /// Parse user id from a proof.
        fn get_user_id(user_auth: Proof) -> ResourceAddress {
            user_auth.unsafe_skip_proof_validation().resource_address()
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, ScryptoSbor)]
pub struct User {
    /// The user's deposit balance
    pub deposit_balance: Decimal,
    /// The interest rate of deposits
    pub deposit_interest_rate: Decimal,
    /// Last update timestamp
    pub deposit_last_update: u64,

    /// The user's borrow balance
    pub borrow_balance: Decimal,
    /// The (stable) interest rate of loans
    pub borrow_interest_rate: Decimal,
    /// Last update timestamp
    pub borrow_last_update: u64,
}

impl User {
    pub fn get_collateral_ratio(&self) -> Option<Decimal> {
        if self.borrow_balance.is_zero() {
            None
        } else {
            let collateral = self.deposit_balance
                + self.deposit_balance * self.deposit_interest_rate * self.deposit_time_elapsed();

            let loan = self.borrow_balance
                + self.borrow_balance * self.borrow_interest_rate * self.borrow_time_elapsed();

            Some(collateral / loan)
        }
    }

    pub fn check_collateral_ratio(&self, min_collateral_ratio: Decimal) {
        let collateral_ratio = self.get_collateral_ratio();
        if let Some(ratio) = collateral_ratio {
            assert!(
                ratio >= min_collateral_ratio,
                "Min collateral ratio does not meet"
            );
        }
    }

    pub fn on_deposit(&mut self, amount: Decimal, interest_rate: Decimal) {
        // Increase principle balance by interests accrued
        let interest =
            self.deposit_balance * self.deposit_interest_rate * self.deposit_time_elapsed();
        self.deposit_balance += interest;
        self.deposit_last_update = Runtime::current_epoch();

        // Calculate the aggregated interest of previous deposits & the new deposit
        self.deposit_interest_rate = (self.deposit_balance * self.deposit_interest_rate
            + amount * interest_rate)
            / (self.deposit_balance + amount);

        // Increase principle balance by the amount.
        self.deposit_balance += amount;
    }

    pub fn on_redeem(&mut self, amount: Decimal) -> Decimal {
        // Deduct withdrawn amount from principle
        self.deposit_balance -= amount;

        // Calculate the amount to return
        amount + amount * self.deposit_interest_rate * self.deposit_time_elapsed()
    }

    pub fn on_borrow(&mut self, amount: Decimal, interest_rate: Decimal) {
        // Increase borrow balance by interests accrued
        let interest = self.borrow_balance * self.borrow_interest_rate * self.borrow_time_elapsed();
        self.borrow_balance += interest;
        self.borrow_last_update = Runtime::current_epoch();

        // Calculate the aggregated interest of previous borrows & the new borrow
        self.borrow_interest_rate = (self.borrow_balance * self.borrow_interest_rate
            + amount * interest_rate)
            / (self.borrow_balance + amount);

        // Increase principle balance by the amount.
        self.borrow_balance += amount;
    }

    pub fn on_repay(&mut self, amount: Decimal) -> Decimal {
        // Increase borrow balance by interests accrued
        let interest = self.borrow_balance * self.borrow_interest_rate * self.borrow_time_elapsed();
        self.borrow_balance += interest;
        self.borrow_last_update = Runtime::current_epoch();

        // Repay the loan
        if self.borrow_balance < amount {
            let to_return = amount - self.borrow_balance;
            self.borrow_balance = Decimal::zero();
            self.borrow_interest_rate = Decimal::zero();
            to_return
        } else {
            self.borrow_balance -= amount;
            Decimal::zero()
        }
    }

    pub fn on_liquidate(&mut self, amount: Decimal, bonus_percent: Decimal) -> Decimal {
        let changes = self.on_repay(amount);
        assert!(changes == 0.into());

        // TODO add exchange rate here when collaterals and borrows are different

        let to_return = amount * (bonus_percent + 1);
        self.deposit_balance -= to_return;
        to_return
    }

    fn deposit_time_elapsed(&self) -> u64 {
        // +1 is for demo purpose only
        Runtime::current_epoch() - self.deposit_last_update + 1
    }

    fn borrow_time_elapsed(&self) -> u64 {
        // +1 is for demo purpose only
        Runtime::current_epoch() - self.borrow_last_update + 1
    }
}
