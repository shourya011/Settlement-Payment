#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, token, Address, Env, String, Symbol,
};

// ─── Storage Keys ────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Settlement,
}

// ─── Settlement Status ───────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub enum SettlementStatus {
    /// Funds have been deposited; awaiting both signatures
    AwaitingSignatures,
    /// Both parties have signed; funds released to recipient
    Released,
    /// Payor cancelled before any signatures were collected
    Cancelled,
}

// ─── Core Settlement Struct ──────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub struct Settlement {
    /// Party that deposited funds (defendant / payor)
    pub payor: Address,
    /// Party that receives the settlement (plaintiff / payee)
    pub payee: Address,
    /// Third-party arbiter / mediator who also must sign
    pub arbiter: Address,
    /// Stellar asset contract address for the payment token
    pub token: Address,
    /// Total amount to be released (in token stroops)
    pub amount: i128,
    /// Human-readable case / agreement reference
    pub case_id: String,
    /// Whether payor has signed the agreement
    pub payor_signed: bool,
    /// Whether payee has signed the agreement
    pub payee_signed: bool,
    /// Current lifecycle status
    pub status: SettlementStatus,
}

// ─── Contract ────────────────────────────────────────────────────────────────

#[contract]
pub struct SettlementContract;

#[contractimpl]
impl SettlementContract {
    // ── Initialise & Fund ────────────────────────────────────────────────────

    /// Called by the payor to initialise and fund the settlement escrow.
    ///
    /// * `payor`   – address that deposits funds (must be the transaction signer)
    /// * `payee`   – address that will receive funds upon dual-signature
    /// * `arbiter` – neutral third-party whose signature is also required
    /// * `token`   – Stellar asset contract for the payment currency
    /// * `amount`  – amount to lock in escrow (in token base units / stroops)
    /// * `case_id` – arbitrary string identifier for the legal agreement
    pub fn initialize(
        env: Env,
        payor: Address,
        payee: Address,
        arbiter: Address,
        token: Address,
        amount: i128,
        case_id: String,
    ) {
        // Ensure this can only be called once
        if env.storage().instance().has(&DataKey::Settlement) {
            panic!("settlement already initialised");
        }

        // Payor must authorise this call
        payor.require_auth();

        // Validate amount
        if amount <= 0 {
            panic!("amount must be positive");
        }

        // Transfer funds from payor into the contract's own account (escrow)
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&payor, &env.current_contract_address(), &amount);

        // Persist settlement state
        let settlement = Settlement {
            payor: payor.clone(),
            payee,
            arbiter,
            token,
            amount,
            case_id,
            payor_signed: false,
            payee_signed: false,
            status: SettlementStatus::AwaitingSignatures,
        };

        env.storage().instance().set(&DataKey::Settlement, &settlement);

        // Emit initialisation event
        env.events().publish(
            (Symbol::new(&env, "settlement_created"),),
            (payor, amount),
        );
    }

    // ── Sign Agreement ───────────────────────────────────────────────────────

    /// Called independently by the payor or payee to record their signature.
    /// Once **both** have signed the funds are automatically released.
    ///
    /// The arbiter signature is collected via `arbiter_release`.
    pub fn sign(env: Env, signer: Address) {
        signer.require_auth();

        let mut settlement: Settlement = env
            .storage()
            .instance()
            .get(&DataKey::Settlement)
            .expect("settlement not found");

        if settlement.status != SettlementStatus::AwaitingSignatures {
            panic!("settlement is no longer open for signatures");
        }

        if signer == settlement.payor {
            if settlement.payor_signed {
                panic!("payor has already signed");
            }
            settlement.payor_signed = true;
        } else if signer == settlement.payee {
            if settlement.payee_signed {
                panic!("payee has already signed");
            }
            settlement.payee_signed = true;
        } else {
            panic!("signer is not a party to this settlement");
        }

        env.storage().instance().set(&DataKey::Settlement, &settlement);

        env.events().publish(
            (Symbol::new(&env, "party_signed"),),
            signer,
        );
    }

    // ── Arbiter Release ──────────────────────────────────────────────────────

    /// Called by the arbiter to confirm the agreement and trigger fund release.
    /// Requires that **both** the payor and payee have already signed.
    pub fn arbiter_release(env: Env, arbiter: Address) {
        arbiter.require_auth();

        let mut settlement: Settlement = env
            .storage()
            .instance()
            .get(&DataKey::Settlement)
            .expect("settlement not found");

        if settlement.status != SettlementStatus::AwaitingSignatures {
            panic!("settlement is not awaiting release");
        }

        if arbiter != settlement.arbiter {
            panic!("caller is not the designated arbiter");
        }

        if !settlement.payor_signed || !settlement.payee_signed {
            panic!("both parties must sign before arbiter can release");
        }

        // Release funds to payee
        let token_client = token::Client::new(&env, &settlement.token);
        token_client.transfer(
            &env.current_contract_address(),
            &settlement.payee,
            &settlement.amount,
        );

        settlement.status = SettlementStatus::Released;
        env.storage().instance().set(&DataKey::Settlement, &settlement);

        env.events().publish(
            (Symbol::new(&env, "settlement_released"),),
            (settlement.payee, settlement.amount),
        );
    }

    // ── Cancel (Payor only, before any signatures) ───────────────────────────

    /// Payor may cancel and reclaim funds only if neither party has signed yet.
    pub fn cancel(env: Env) {
        let mut settlement: Settlement = env
            .storage()
            .instance()
            .get(&DataKey::Settlement)
            .expect("settlement not found");

        settlement.payor.require_auth();

        if settlement.status != SettlementStatus::AwaitingSignatures {
            panic!("cannot cancel: settlement is no longer open");
        }

        if settlement.payor_signed || settlement.payee_signed {
            panic!("cannot cancel after signatures have been collected");
        }

        // Refund payor
        let token_client = token::Client::new(&env, &settlement.token);
        token_client.transfer(
            &env.current_contract_address(),
            &settlement.payor,
            &settlement.amount,
        );

        settlement.status = SettlementStatus::Cancelled;
        env.storage().instance().set(&DataKey::Settlement, &settlement);

        env.events().publish(
            (Symbol::new(&env, "settlement_cancelled"),),
            settlement.payor,
        );
    }

    // ── Read-only Queries ────────────────────────────────────────────────────

    /// Returns the full settlement record.
    pub fn get_settlement(env: Env) -> Settlement {
        env.storage()
            .instance()
            .get(&DataKey::Settlement)
            .expect("settlement not found")
    }

    /// Returns the current status of the settlement.
    pub fn get_status(env: Env) -> SettlementStatus {
        let settlement: Settlement = env
            .storage()
            .instance()
            .get(&DataKey::Settlement)
            .expect("settlement not found");
        settlement.status
    }

    /// Returns whether both parties have signed.
    pub fn is_fully_signed(env: Env) -> bool {
        let settlement: Settlement = env
            .storage()
            .instance()
            .get(&DataKey::Settlement)
            .expect("settlement not found");
        settlement.payor_signed && settlement.payee_signed
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation, Events},
        token::{Client as TokenClient, StellarAssetClient},
        vec, Address, Env, IntoVal, String,
    };

    fn create_token<'a>(env: &Env, admin: &Address) -> (TokenClient<'a>, StellarAssetClient<'a>) {
        let contract_address = env.register_stellar_asset_contract_v2(admin.clone());
        (
            TokenClient::new(env, &contract_address.address()),
            StellarAssetClient::new(env, &contract_address.address()),
        )
    }

    #[test]
    fn test_full_happy_path() {
        let env = Env::default();
        env.mock_all_auths();

        let payor   = Address::generate(&env);
        let payee   = Address::generate(&env);
        let arbiter = Address::generate(&env);

        let (token, token_admin) = create_token(&env, &payor);
        token_admin.mint(&payor, &10_000);

        let contract_id = env.register_contract(None, SettlementContract);
        let client = SettlementContractClient::new(&env, &contract_id);

        // Initialise escrow
        client.initialize(
            &payor,
            &payee,
            &arbiter,
            &token.address,
            &5_000,
            &String::from_str(&env, "CASE-2024-001"),
        );

        assert_eq!(token.balance(&payor), 5_000);
        assert_eq!(token.balance(&contract_id), 5_000);

        // Both parties sign
        client.sign(&payor);
        client.sign(&payee);

        assert!(client.is_fully_signed());

        // Arbiter releases
        client.arbiter_release(&arbiter);

        assert_eq!(token.balance(&payee), 5_000);
        assert_eq!(token.balance(&contract_id), 0);
        assert_eq!(client.get_status(), SettlementStatus::Released);
    }

    #[test]
    #[should_panic(expected = "both parties must sign before arbiter can release")]
    fn test_arbiter_cannot_release_without_both_signatures() {
        let env = Env::default();
        env.mock_all_auths();

        let payor   = Address::generate(&env);
        let payee   = Address::generate(&env);
        let arbiter = Address::generate(&env);

        let (token, token_admin) = create_token(&env, &payor);
        token_admin.mint(&payor, &10_000);

        let contract_id = env.register_contract(None, SettlementContract);
        let client = SettlementContractClient::new(&env, &contract_id);

        client.initialize(
            &payor, &payee, &arbiter,
            &token.address, &5_000,
            &String::from_str(&env, "CASE-2024-002"),
        );

        // Only payor signs – payee hasn't yet
        client.sign(&payor);
        client.arbiter_release(&arbiter); // must panic
    }

    #[test]
    fn test_cancel_refunds_payor() {
        let env = Env::default();
        env.mock_all_auths();

        let payor   = Address::generate(&env);
        let payee   = Address::generate(&env);
        let arbiter = Address::generate(&env);

        let (token, token_admin) = create_token(&env, &payor);
        token_admin.mint(&payor, &10_000);

        let contract_id = env.register_contract(None, SettlementContract);
        let client = SettlementContractClient::new(&env, &contract_id);

        client.initialize(
            &payor, &payee, &arbiter,
            &token.address, &5_000,
            &String::from_str(&env, "CASE-2024-003"),
        );

        client.cancel();

        assert_eq!(token.balance(&payor), 10_000);
        assert_eq!(client.get_status(), SettlementStatus::Cancelled);
    }
}