use scrypto::prelude::*;

// This is a simple Airdrop blueprint. All components instantiated from it will initially
// hold 1000 FreeToken within a vault. When the `free_token` method is called, 1 FreeToken will be
// taken from the vault and returned to the caller.

blueprint! {
    struct Airdrop {
        tokens: Vault,
    }

    impl Airdrop {
        pub fn instantiate_airdrop() -> ComponentAddress {
            Self {
                tokens: Vault::with_bucket(
                    ResourceBuilder::new_fungible()
                        .divisibility(DIVISIBILITY_MAXIMUM)
                        .metadata("name", "FreeToken")
                        .initial_supply(1000),
                ),
            }
            .instantiate()
            .globalize()
        }

        pub fn free_token(&mut self) -> Bucket {
            // Take 1 FreeToken and return
            self.tokens.take(1)
        }
    }
}
