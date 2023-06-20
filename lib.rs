#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod erc20 {
    use ink::storage::Mapping;

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    #[derive(Default)]
    pub struct Erc20 {
        total_supply: Balance,
        balances: Mapping<AccountId, Balance>,
        allowances: Mapping<(AccountId, AccountId), Balance>,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        BalanceTooLow,
        AllowanceTooLow,
    }

    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: AccountId,
        #[ink(topic)]
        value: Balance,
    }

    #[ink(event)]
    pub struct Approve {
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
        #[ink(topic)]
        value: Balance,
    }

    type Result<T> = core::result::Result<T, Error>;
    impl Erc20 {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new(total_supply: Balance) -> Self {
            let mut balances = Mapping::new();
            let sender = Self::env().caller();
            balances.insert(&sender, &total_supply);
            Self::env().emit_event(Transfer {
                from: None,
                to: sender,
                value: total_supply,
            });
            Self {
                total_supply,
                balances,
                ..Default::default()
            }
        }

        #[ink(message)]
        pub fn total_supply(&self) -> Balance {
            self.total_supply
        }

        /// Simply returns the current value of our `bool`.
        #[ink(message)]
        pub fn balance_of(&self, who: AccountId) -> Balance {
            self.balances.get(&who).unwrap_or_default()
        }

        #[ink(message)]
        pub fn transfer(&mut self, to: AccountId, value: Balance) -> Result<()> {
            let sender = self.env().caller();
            return self.transfer_from_to(&sender, &to, value);
        }

        #[ink(message)]
        pub fn transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            value: Balance,
        ) -> Result<()> {
            let sender = self.env().caller();
            let allowance = self.allowances.get(&(from, sender)).unwrap_or_default();
            if allowance < value {
                return Err(Error::AllowanceTooLow);
            }
            self.allowances
                .insert(&(from, sender), &(allowance - value));
            self.transfer_from_to(&from, &to, value)?;
            Ok(())
        }

        fn transfer_from_to(
            &mut self,
            from: &AccountId,
            to: &AccountId,
            value: Balance,
        ) -> Result<()> {
            let balance_from = self.balance_of(*from);
            let balance_to = self.balance_of(*to);
            if value > balance_from {
                return Err(Error::BalanceTooLow);
            }
            self.balances.insert(from, &(balance_from - value));
            self.balances.insert(to, &(balance_to + value));
            self.env().emit_event({
                Transfer {
                    from: Some(*from),
                    to: *to,
                    value,
                }
            });

            Ok(())
        }

        #[ink(message)]
        pub fn approve(&mut self, to: AccountId, value: Balance) -> Result<()> {
            let sender = self.env().caller();
            self.allowances.insert(&(sender, to), &value);
            self.env().emit_event(Approve {
                from: sender,
                to,
                value,
            });
            Ok(())
        }

        #[ink(message)]
        pub fn allowance(&self, from: AccountId, to: AccountId) -> Balance {
            self.allowances.get(&(from, to)).unwrap_or_default()
        }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;
        type Event = <Erc20 as ::ink::reflect::ContractEventBase>::Type;
        use ink::env::{test, DefaultEnvironment};
        #[ink::test]
        fn constructor_works() {
            let total_supply = 10_000;
            let erc20 = Erc20::new(total_supply);
            let accounts = test::default_accounts::<DefaultEnvironment>();
            assert_eq!(erc20.total_supply, total_supply);
            assert_eq!(erc20.balance_of(accounts.alice), total_supply);

            let emitted_events = test::recorded_events().collect::<Vec<_>>();
            let event = &emitted_events[0];
            let decoded =
                <Event as scale::Decode>::decode(&mut &event.data[..]).expect("decoded error");
            match decoded {
                Event::Transfer(Transfer { from, to, value }) => {
                    assert!(from.is_none());
                    assert_eq!(to, accounts.alice);
                    assert_eq!(value, total_supply);
                }
                _ => panic!("Event do not match"),
            }
        }

        #[ink::test]
        fn transfer_should_work() {
            let total_supply = 10_000;
            let transfer_amount = 1_000;
            let mut erc20 = Erc20::new(total_supply);
            let accounts = test::default_accounts::<DefaultEnvironment>();
            let res = erc20.transfer(accounts.bob, transfer_amount);

            assert!(res.is_ok());
            assert_eq!(
                erc20.balance_of(accounts.alice),
                total_supply - transfer_amount
            );
            assert_eq!(erc20.balance_of(accounts.bob), transfer_amount);
        }

        #[ink::test]
        fn invalid_transfer_should_fail() {
            let total_supply = 10_000;
            let transfer_amount = 1_000;
            let mut erc20 = Erc20::new(total_supply);
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            let res = erc20.transfer(accounts.bob, transfer_amount);
            assert_eq!(res, Err(Error::BalanceTooLow));
        }

        #[ink::test]
        fn approve_then_transfer_should_work() {
            let total_supply = 10_000;
            let approve_amount = 1_000;
            let transfer_amount = 1_000;
            let mut erc20 = Erc20::new(total_supply);
            let accounts = test::default_accounts::<DefaultEnvironment>();
            let res = erc20.approve(accounts.bob, approve_amount);
            assert!(res.is_ok());
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            let res = erc20.transfer_from(accounts.alice, accounts.charlie, transfer_amount);
            assert!(res.is_ok());
            let allowance = erc20.allowance(accounts.alice, accounts.bob);
            assert_eq!(allowance, approve_amount - transfer_amount);
        }

        #[ink::test]
        fn transfer_from_failed_when_allowance_too_low() {
            let total_supply = 10_000;
            let approve_amount = 999;
            let transfer_amount = 1_000;
            let mut erc20 = Erc20::new(total_supply);
            let accounts = test::default_accounts::<DefaultEnvironment>();
            let res = erc20.approve(accounts.bob, approve_amount);
            assert!(res.is_ok());
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            let res = erc20.transfer_from(accounts.alice, accounts.charlie, transfer_amount);
            assert_eq!(res, Err(Error::AllowanceTooLow));
        }
    }

    /// This is how you'd write end-to-end (E2E) or integration tests for ink! contracts.
    ///
    /// When running these you need to make sure that you:
    /// - Compile the tests with the `e2e-tests` feature flag enabled (`--features e2e-tests`)
    /// - Are running a Substrate node which contains `pallet-contracts` in the background
    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// A helper function used for calling contract messages.
        use ink_e2e::build_message;

        /// The End-to-End test `Result` type.
        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        /// We test that we can upload and instantiate the contract using its default constructor.
        #[ink_e2e::test]
        async fn e2e_transfer_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            let total_supply = 100_000;
            let transfer_amount = 1_000;
            let constructor = Erc20Ref::new(total_supply);

            let contract_account_id = client
                .instantiate("erc20", &ink_e2e::alice(), constructor, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;

            let alice_acc = ink_e2e::account_id(ink_e2e::AccountKeyring::Alice);
            let bob_acc = ink_e2e::account_id(ink_e2e::AccountKeyring::Bob);

            let transfer_msg = build_message::<Erc20Ref>(contract_account_id.clone())
                .call(|erc20| erc20.transfer(bob_acc, transfer_amount));

            let res = client.call(&ink_e2e::alice(), transfer_msg, 0, None).await;
            assert!(res.is_ok());

            let balance_of_msg = build_message::<Erc20Ref>(contract_account_id.clone())
                .call(|erc20| erc20.balance_of(alice_acc));

            let res = client
                .call_dry_run(&ink_e2e::alice(), &balance_of_msg, 0, None)
                .await;
            assert_eq!(res.return_value(), total_supply - transfer_amount);

            Ok(())
        }

        #[ink_e2e::test]
        async fn e2e_approve_then_transfer_from_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            let total_supply = 100_000;
            let transfer_amount = 1_000;
            let constructor = Erc20Ref::new(total_supply);

            let contract_account_id = client
                .instantiate("erc20", &ink_e2e::alice(), constructor, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;

            let alice_acc = ink_e2e::account_id(ink_e2e::AccountKeyring::Alice);
            let bob_acc = ink_e2e::account_id(ink_e2e::AccountKeyring::Bob);
            let charlie_acc = ink_e2e::account_id(ink_e2e::AccountKeyring::Charlie);

            let approve_msg = build_message::<Erc20Ref>(contract_account_id.clone())
                .call(|erc20| erc20.approve(bob_acc, transfer_amount));
            let res = client.call(&ink_e2e::alice(), approve_msg, 0, None).await;
            assert!(res.is_ok());

            let transfer_from_msg = build_message::<Erc20Ref>(contract_account_id.clone())
                .call(|erc20| erc20.transfer_from(alice_acc, charlie_acc, transfer_amount));
            let res = client.call(&ink_e2e::bob(), transfer_from_msg, 0, None).await;
            assert!(res.is_ok());

            let balance_of_msg = build_message::<Erc20Ref>(contract_account_id.clone())
                .call(|erc20| erc20.balance_of(alice_acc));
            let res = client
                .call_dry_run(&ink_e2e::alice(), &balance_of_msg, 0, None)
                .await;
            assert_eq!(res.return_value(), total_supply - transfer_amount);
            
            let allowance_msg = build_message::<Erc20Ref>(contract_account_id.clone())
                .call(|erc20| erc20.allowance(alice_acc, bob_acc));
            let res = client
                .call_dry_run(&ink_e2e::alice(), &allowance_msg, 0, None)
                .await;//.expect("query allowance error");
            assert_eq!(res.return_value(), 0);

            Ok(())
        }
    }
}
