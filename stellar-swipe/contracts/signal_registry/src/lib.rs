#![no_std]

mod admin;
mod errors;
#[allow(deprecated)]
mod events;
#[allow(dead_code)]
mod expiry;
#[allow(dead_code)]
mod fees;
mod import;
mod leaderboard;
mod performance;
mod query;
mod social;
mod stake;
mod submission;
pub mod templates;
mod types;

use admin::{
    get_admin, get_admin_config, get_pause_info, init_admin, is_trading_paused, require_not_paused,
    AdminConfig, PauseInfo,
};
use errors::{AdminError, TemplateError};
pub use leaderboard::{get_leaderboard, LeaderboardMetric, ProviderLeaderboard};
use soroban_sdk::{contract, contractimpl, contracttype, Address, Bytes, Env, Map, String, Vec};
use stellar_swipe_common::{validate_asset_pair as validate_asset_pair_common, AssetPairError};
use templates::{SignalTemplate, DEFAULT_TEMPLATE_EXPIRY_HOURS};
use types::{
    Asset, FeeBreakdown, ImportResultView, ProviderPerformance, Signal, SignalAction,
    SignalPerformanceView, SignalStatus, SignalSummary, SortOption, TradeExecution,
};

const MAX_EXPIRY_SECONDS: u64 = 30 * 24 * 60 * 60;

#[contract]
pub struct SignalRegistry;

#[contracttype]
#[derive(Clone)]
pub enum StorageKey {
    SignalCounter,
    Signals,
    ProviderStats,
    TradeExecutions,
    TemplateCounter,
    Templates,
    ExternalIdMappings,
}

#[contractimpl]
impl SignalRegistry {
    /* =========================
       INITIALIZATION
    ========================== */

    /// Initialize contract with admin
    pub fn initialize(env: Env, admin: Address) -> Result<(), AdminError> {
        init_admin(&env, admin)
    }

    /* =========================
       ADMIN FUNCTIONS
    ========================== */

    pub fn set_min_stake(env: Env, caller: Address, new_amount: i128) -> Result<(), AdminError> {
        admin::set_min_stake(&env, &caller, new_amount)
    }

    pub fn set_trade_fee(env: Env, caller: Address, new_fee_bps: u32) -> Result<(), AdminError> {
        admin::set_trade_fee(&env, &caller, new_fee_bps)
    }

    pub fn set_risk_defaults(
        env: Env,
        caller: Address,
        stop_loss: u32,
        position_limit: u32,
    ) -> Result<(), AdminError> {
        admin::set_risk_defaults(&env, &caller, stop_loss, position_limit)
    }

    pub fn pause_trading(env: Env, caller: Address) -> Result<(), AdminError> {
        admin::pause_trading(&env, &caller)
    }

    pub fn unpause_trading(env: Env, caller: Address) -> Result<(), AdminError> {
        admin::unpause_trading(&env, &caller)
    }

    pub fn transfer_admin(env: Env, caller: Address, new_admin: Address) -> Result<(), AdminError> {
        admin::transfer_admin(&env, &caller, new_admin)
    }

    pub fn get_admin(env: Env) -> Result<Address, AdminError> {
        get_admin(&env)
    }

    pub fn get_config(env: Env) -> AdminConfig {
        get_admin_config(&env)
    }

    pub fn is_paused(env: Env) -> bool {
        is_trading_paused(&env)
    }

    pub fn get_pause_info(env: Env) -> PauseInfo {
        get_pause_info(&env)
    }

    // Multi-sig functions
    pub fn enable_multisig(
        env: Env,
        caller: Address,
        signers: Vec<Address>,
        threshold: u32,
    ) -> Result<(), AdminError> {
        admin::enable_multisig(&env, &caller, signers, threshold)
    }

    pub fn disable_multisig(env: Env, caller: Address) -> Result<(), AdminError> {
        admin::disable_multisig(&env, &caller)
    }

    pub fn is_multisig_enabled(env: Env) -> bool {
        admin::is_multisig_enabled(&env)
    }

    pub fn get_multisig_signers(env: Env) -> Vec<Address> {
        admin::get_multisig_signers(&env)
    }

    pub fn get_multisig_threshold(env: Env) -> u32 {
        admin::get_multisig_threshold(&env)
    }

    pub fn add_multisig_signer(
        env: Env,
        caller: Address,
        new_signer: Address,
    ) -> Result<(), AdminError> {
        admin::add_multisig_signer(&env, &caller, new_signer)
    }

    pub fn remove_multisig_signer(
        env: Env,
        caller: Address,
        signer_to_remove: Address,
    ) -> Result<(), AdminError> {
        admin::remove_multisig_signer(&env, &caller, signer_to_remove)
    }

    /* =========================
       INTERNAL HELPERS
    ========================== */

    fn next_signal_id(env: &Env) -> u64 {
        let mut counter: u64 = env
            .storage()
            .instance()
            .get(&StorageKey::SignalCounter)
            .unwrap_or(0);

        counter = counter.checked_add(1).expect("signal id overflow");

        env.storage()
            .instance()
            .set(&StorageKey::SignalCounter, &counter);

        counter
    }

    fn get_signals_map(env: &Env) -> Map<u64, Signal> {
        env.storage()
            .instance()
            .get(&StorageKey::Signals)
            .unwrap_or(Map::new(env))
    }

    fn save_signals_map(env: &Env, map: &Map<u64, Signal>) {
        env.storage().instance().set(&StorageKey::Signals, map);
    }

    fn get_provider_stats_map(env: &Env) -> Map<Address, ProviderPerformance> {
        env.storage()
            .instance()
            .get(&StorageKey::ProviderStats)
            .unwrap_or(Map::new(env))
    }

    fn save_provider_stats_map(env: &Env, map: &Map<Address, ProviderPerformance>) {
        env.storage()
            .instance()
            .set(&StorageKey::ProviderStats, map);
    }

    fn validate_asset_pair(env: &Env, asset_pair: &String) -> Result<(), AdminError> {
        validate_asset_pair_common(env, asset_pair).map_err(|e| match e {
            AssetPairError::InvalidFormat
            | AssetPairError::InvalidAssetCode
            | AssetPairError::InvalidIssuer
            | AssetPairError::SameAssets => AdminError::InvalidAssetPair,
        })
    }

    /* =========================
       PUBLIC API
    ========================== */

    pub fn create_signal(
        env: Env,
        provider: Address,
        asset_pair: String,
        action: SignalAction,
        price: i128,
        rationale: String,
        expiry: u64,
    ) -> Result<u64, AdminError> {
        provider.require_auth();
        Self::create_signal_internal(&env, provider, asset_pair, action, price, rationale, expiry)
    }

    fn create_signal_internal(
        env: &Env,
        provider: Address,
        asset_pair: String,
        action: SignalAction,
        price: i128,
        rationale: String,
        expiry: u64,
    ) -> Result<u64, AdminError> {
        // Check if trading is paused
        require_not_paused(env)?;

        Self::validate_asset_pair(env, &asset_pair)?;

        let now = env.ledger().timestamp();

        if expiry <= now {
            panic!("expiry must be in the future");
        }

        if expiry > now + MAX_EXPIRY_SECONDS {
            panic!("expiry exceeds max 30 days");
        }

        let id = Self::next_signal_id(env);

        let signal = Signal {
            id,
            provider: provider.clone(),
            asset_pair,
            action,
            price,
            rationale,
            timestamp: now,
            expiry,
            status: SignalStatus::Active,
            // Initialize performance tracking fields
            executions: 0,
            successful_executions: 0,
            total_volume: 0,
            total_roi: 0,
        };

        // Store signal
        let mut signals = Self::get_signals_map(env);
        signals.set(id, signal);
        Self::save_signals_map(env, &signals);

        // Initialize provider stats on first submission
        let mut stats = Self::get_provider_stats_map(env);
        if !stats.contains_key(provider.clone()) {
            stats.set(provider, ProviderPerformance::default());
            Self::save_provider_stats_map(env, &stats);
        }

        Ok(id)
    }

    pub fn get_signal(env: Env, signal_id: u64) -> Option<Signal> {
        let signals = Self::get_signals_map(&env);
        signals.get(signal_id)
    }

    pub fn get_provider_stats(env: Env, provider: Address) -> Option<ProviderPerformance> {
        let stats = Self::get_provider_stats_map(&env);
        stats.get(provider)
    }

    pub fn create_template(
        env: Env,
        provider: Address,
        name: String,
        asset_pair: Option<String>,
        rationale_template: String,
    ) -> Result<u64, TemplateError> {
        provider.require_auth();

        if name.len() == 0 || rationale_template.len() == 0 {
            return Err(TemplateError::InvalidTemplate);
        }

        if let Some(ref pair) = asset_pair {
            Self::validate_asset_pair(&env, pair).map_err(|_| TemplateError::InvalidTemplate)?;
        }

        let template_id = templates::get_next_template_id(&env);

        let template = SignalTemplate {
            id: template_id,
            provider: provider.clone(),
            name,
            asset_pair,
            action: None,
            rationale_template,
            default_expiry_hours: DEFAULT_TEMPLATE_EXPIRY_HOURS,
            is_public: false,
            use_count: 0,
        };

        templates::store_template(&env, template_id, &template);
        Ok(template_id)
    }

    pub fn set_template_public(
        env: Env,
        provider: Address,
        template_id: u64,
        is_public: bool,
    ) -> Result<(), TemplateError> {
        provider.require_auth();
        templates::set_template_visibility(&env, &provider, template_id, is_public)
    }

    pub fn get_template(env: Env, template_id: u64) -> Option<SignalTemplate> {
        templates::get_template(&env, template_id)
    }

    pub fn submit_from_template(
        env: Env,
        submitter: Address,
        template_id: u64,
        variables: Map<String, String>,
    ) -> Result<u64, TemplateError> {
        submitter.require_auth();

        let template =
            templates::get_template(&env, template_id).ok_or(TemplateError::TemplateNotFound)?;
        if !template.is_public && template.provider != submitter {
            return Err(TemplateError::PrivateTemplate);
        }

        let asset_pair = match template.asset_pair {
            Some(pair) => pair,
            None => templates::get_variable(&variables, "asset_pair")?
                .ok_or(TemplateError::MissingVariable)?,
        };
        Self::validate_asset_pair(&env, &asset_pair).map_err(|_| TemplateError::InvalidTemplate)?;

        let action = match template.action {
            Some(template_action) => templates::parse_action(&template_action)?,
            None => {
                let action_text = templates::get_variable(&variables, "action")?
                    .ok_or(TemplateError::MissingVariable)?;
                templates::parse_action(&action_text)?
            }
        };

        let price_text =
            templates::get_variable(&variables, "price")?.ok_or(TemplateError::MissingVariable)?;
        let price = templates::parse_price(&price_text)?;

        let rationale =
            templates::replace_variables(&env, &template.rationale_template, &variables)?;

        let expiry = env
            .ledger()
            .timestamp()
            .checked_add((template.default_expiry_hours as u64) * 60 * 60)
            .ok_or(TemplateError::InvalidExpiry)?;
        if expiry > env.ledger().timestamp() + MAX_EXPIRY_SECONDS {
            return Err(TemplateError::InvalidExpiry);
        }

        let signal_id = Self::create_signal_internal(
            &env, submitter, asset_pair, action, price, rationale, expiry,
        )
        .map_err(|_| TemplateError::InvalidTemplate)?;

        templates::increment_template_use_count(&env, template_id)?;
        Ok(signal_id)
    }

    /* =========================
       PERFORMANCE TRACKING FUNCTIONS
    ========================== */

    /// Record a trade execution for a signal and update performance stats
    pub fn record_trade_execution(
        env: Env,
        executor: Address,
        signal_id: u64,
        entry_price: i128,
        exit_price: i128,
        volume: i128,
    ) -> Result<(), errors::PerformanceError> {
        // Require executor authorization
        executor.require_auth();

        // Validate inputs
        if entry_price <= 0 || exit_price <= 0 {
            return Err(errors::PerformanceError::InvalidPrice);
        }
        if volume <= 0 {
            return Err(errors::PerformanceError::InvalidVolume);
        }

        // Load signal
        let mut signals = Self::get_signals_map(&env);
        let mut signal = signals
            .get(signal_id)
            .ok_or(errors::PerformanceError::SignalNotFound)?;

        // Calculate ROI
        let roi = performance::calculate_roi(entry_price, exit_price, &signal.action);

        // Create trade execution record
        let trade = TradeExecution {
            signal_id,
            executor: executor.clone(),
            entry_price,
            exit_price,
            volume,
            roi,
            timestamp: env.ledger().timestamp(),
        };

        // Store old status for comparison
        let old_status = signal.status.clone();

        // Update signal stats
        performance::update_signal_stats(&mut signal, &trade);

        // Evaluate new status
        let now = env.ledger().timestamp();
        let new_status = performance::evaluate_signal_status(&signal, now);
        signal.status = new_status.clone();

        // Save updated signal
        signals.set(signal_id, signal.clone());
        Self::save_signals_map(&env, &signals);

        // Emit trade executed event
        events::emit_trade_executed(&env, signal_id, executor.clone(), roi, volume);

        // Check if status changed and update provider stats
        if performance::should_update_provider_stats(&old_status, &new_status) {
            let mut provider_stats_map = Self::get_provider_stats_map(&env);
            let mut provider_stats = provider_stats_map
                .get(signal.provider.clone())
                .unwrap_or_default();

            let signal_avg_roi = performance::get_signal_average_roi(&signal);

            performance::update_provider_performance(
                &mut provider_stats,
                &old_status,
                &new_status,
                signal_avg_roi,
                signal.total_volume,
            );

            provider_stats_map.set(signal.provider.clone(), provider_stats.clone());
            Self::save_provider_stats_map(&env, &provider_stats_map);

            // Emit status change event
            events::emit_signal_status_changed(
                &env,
                signal_id,
                signal.provider.clone(),
                old_status as u32,
                new_status as u32,
            );

            // Emit provider stats updated event
            events::emit_provider_stats_updated(
                &env,
                signal.provider,
                provider_stats.success_rate,
                provider_stats.avg_return,
                provider_stats.total_volume,
            );
        }

        Ok(())
    }

    /// Get signal performance metrics
    pub fn get_signal_performance(env: Env, signal_id: u64) -> Option<SignalPerformanceView> {
        let signals = Self::get_signals_map(&env);
        let signal = signals.get(signal_id)?;

        let average_roi = performance::get_signal_average_roi(&signal);

        Some(SignalPerformanceView {
            signal_id: signal.id,
            executions: signal.executions,
            total_volume: signal.total_volume,
            average_roi,
            status: signal.status,
        })
    }

    /// Get provider performance stats (alias for get_provider_stats)
    pub fn get_provider_performance(env: Env, provider: Address) -> Option<ProviderPerformance> {
        Self::get_provider_stats(env, provider)
    }

    /// Get leaderboard of top providers by metric
    ///
    /// # Arguments
    /// * `metric` - SuccessRate, Volume, or Followers (empty for MVP)
    /// * `limit` - Max providers to return (0 = default 10, max 50)
    ///
    /// # Minimum qualification
    /// - >= 5 signals with terminal status
    /// - success_rate > 0 (exclude all-failed)
    pub fn get_leaderboard(
        env: Env,
        metric: LeaderboardMetric,
        limit: u32,
    ) -> Vec<ProviderLeaderboard> {
        let stats_map = Self::get_provider_stats_map(&env);
        get_leaderboard(&env, &stats_map, metric, limit)
    }

    /// Get top providers sorted by success rate
    pub fn get_top_providers(env: Env, limit: u32) -> Vec<(Address, ProviderPerformance)> {
        let stats_map = Self::get_provider_stats_map(&env);
        let mut providers = Vec::new(&env);

        // Collect all providers
        for key in stats_map.keys() {
            if let Some(stats) = stats_map.get(key.clone()) {
                providers.push_back((key, stats));
            }
        }

        // Sort by success rate (descending)
        // Note: Soroban Vec doesn't have built-in sort, so we implement a simple bubble sort
        let len = providers.len();
        for i in 0..len {
            for j in 0..(len - i - 1) {
                let curr = providers.get(j).unwrap();
                let next = providers.get(j + 1).unwrap();

                if curr.1.success_rate < next.1.success_rate {
                    // Swap
                    let temp = curr.clone();
                    providers.set(j, next);
                    providers.set(j + 1, temp);
                }
            }
        }

        // Return top N
        let result_len = if limit < len { limit } else { len };
        let mut result = Vec::new(&env);
        for i in 0..result_len {
            result.push_back(providers.get(i).unwrap());
        }

        result
    }

    /* =========================
       FEE MANAGEMENT FUNCTIONS
    ========================== */

    pub fn set_platform_treasury(
        env: Env,
        caller: Address,
        treasury: Address,
    ) -> Result<(), AdminError> {
        admin::require_admin(&env, &caller)?;
        caller.require_auth();
        fees::set_platform_treasury(&env, treasury);
        Ok(())
    }

    pub fn get_platform_treasury(env: Env) -> Option<Address> {
        fees::get_platform_treasury(&env)
    }

    pub fn get_treasury_balance(env: Env, asset: Asset) -> i128 {
        fees::get_treasury_balance(&env, asset)
    }

    pub fn get_all_treasury_balances(env: Env) -> Map<Asset, i128> {
        fees::get_all_treasury_balances(&env)
    }

    pub fn calculate_fee_preview(
        _env: Env,
        trade_amount: i128,
    ) -> Result<FeeBreakdown, errors::FeeError> {
        fees::calculate_fee_breakdown(trade_amount)
    }

    /* =========================
       API: QUERY SIGNALS
    ========================== */

    /// Get all active (non-expired) signals for feed, paginated and sorted.
    pub fn get_active_signals(
        env: Env,
        offset: u32,
        limit: u32,
        sort_by: SortOption,
        provider: Option<Address>,
    ) -> Vec<SignalSummary> {
        let signals_map = Self::get_signals_map(&env);
        query::get_active_signals(&env, &signals_map, provider, offset, limit, sort_by)
    }

    /// Legacy fallback if front-ends rely on Old behavior
    /// (Wait, let's keep it as another name if needed, or just let users migrate to the new `get_active_signals`)
    pub fn get_active_signals_archived(
        env: Env,
        user: Address,
        followed_only: bool,
    ) -> Vec<Signal> {
        let signals = Self::get_signals_map(&env);
        if followed_only {
            let followed = social::get_followed_providers(&env, &user);
            expiry::get_active_signals_filtered(&env, &signals, &followed)
        } else {
            expiry::get_active_signals(&env, &signals)
        }
    }

    /* =========================
       SOCIAL / FOLLOW FUNCTIONS
    ========================== */

    /// Follow a provider. Idempotent if already following.
    pub fn follow_provider(env: Env, user: Address, provider: Address) -> Result<(), AdminError> {
        social::follow_provider(&env, user, provider).map_err(|_| AdminError::CannotFollowSelf)
    }

    /// Unfollow a provider. No error if not following.
    pub fn unfollow_provider(env: Env, user: Address, provider: Address) -> Result<(), AdminError> {
        social::unfollow_provider(&env, user, provider).map_err(|_| AdminError::Unauthorized)
    }

    /// Get list of providers user follows
    pub fn get_followed_providers(env: Env, user: Address) -> Vec<Address> {
        social::get_followed_providers(&env, &user)
    }

    /// Get follower count for a provider
    pub fn get_follower_count(env: Env, provider: Address) -> u32 {
        social::get_follower_count(&env, &provider)
    }

    /// Cleanup expired signals in batches
    /// Returns (signals_processed, signals_expired)
    pub fn cleanup_expired_signals(env: Env, limit: u32) -> (u32, u32) {
        let signals = Self::get_signals_map(&env);
        let result = expiry::cleanup_expired_signals(&env, &signals, limit);
        (result.signals_processed, result.signals_expired)
    }

    /// Archive old expired signals (30+ days old)
    /// Returns number of signals archived
    pub fn archive_old_signals(env: Env, limit: u32) -> u32 {
        let signals = Self::get_signals_map(&env);
        expiry::archive_old_signals(&env, &signals, limit)
    }

    /// Get count of expired signals
    pub fn get_expired_count(env: Env) -> u32 {
        let signals = Self::get_signals_map(&env);
        expiry::count_expired_signals(&signals)
    }

    /// Get count of signals pending expiry (past expiry time but not marked yet)
    pub fn get_pending_expiry_count(env: Env) -> u32 {
        let signals = Self::get_signals_map(&env);
        expiry::count_signals_pending_expiry(&env, &signals)
    }

    /* =========================
       SIGNAL IMPORT FUNCTIONS
    ========================== */

    /// Import signals from CSV format
    pub fn import_signals_csv(
        env: Env,
        provider: Address,
        data: Bytes,
        validate_only: bool,
    ) -> ImportResultView {
        provider.require_auth();

        let result = import::import_signals_csv(&env, &provider, data, validate_only);

        ImportResultView {
            success_count: result.success_count,
            error_count: result.error_count,
            signal_ids: Vec::new(&env),
        }
    }

    /// Import signals from JSON format
    pub fn import_signals_json(
        env: Env,
        provider: Address,
        data: Bytes,
        validate_only: bool,
    ) -> ImportResultView {
        provider.require_auth();

        let result = import::import_signals_json(&env, &provider, data, validate_only);

        ImportResultView {
            success_count: result.success_count,
            error_count: result.error_count,
            signal_ids: Vec::new(&env),
        }
    }

    /// Get signal ID by external ID
    pub fn get_signal_by_external_id(
        env: Env,
        provider: Address,
        external_id: String,
    ) -> Option<u64> {
        import::get_signal_by_external_id(&env, &provider, &external_id)
    }
}

mod test;
mod test_import;
mod test_performance;
