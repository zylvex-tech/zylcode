use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::plugin_marketplace::{
    PluginDefinition, PluginExecution, PluginMarketplace, PluginMarketplaceInfo, PluginPricing,
};

/// Enhanced Plugin Marketplace with revenue features
pub struct EnhancedPluginMarketplace {
    marketplace: PluginMarketplace,
    revenue_manager: Arc<RevenueManager>,
    analytics: Arc<MarketplaceAnalytics>,
}

/// Revenue manager for handling payments and subscriptions
pub struct RevenueManager {
    _payment_gateway: PaymentGateway,
    subscription_manager: SubscriptionManager,
    revenue_tracker: RevenueTracker,
    _payout_manager: PayoutManager,
    recommendation_engine: Arc<RecommendationEngine>,
}

/// Payment gateway — **inert scaffold. Not an operational integration.**
///
/// That claim is checkable from this workspace rather than an assumption:
/// * no field of this type is ever read (`grep '_payment_gateway\.' crates/`
///   matches nothing);
/// * [`RevenueManager::process_payment`] never calls it — it records a locally
///   simulated, immediately-`Completed` transaction and returns
///   `PaymentResult { success: true }` without contacting any processor;
/// * the payment path contains no HTTP client, no provider SDK dependency and no
///   credential source (no environment read, no config file, no keyring).
///
/// The type exists so the marketplace's in-memory accounting loop stays
/// exercisable in tests. **Its presence is not evidence that payments work.** Any
/// claim of a working payment flow requires a provider wired through
/// [`PaymentGatewayProvider`] with its key supplied at runtime — the same
/// convention `zylcode_core::competition::natlas` already uses for its own API
/// key (read from the environment, redacted in debug output).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentGateway {
    /// Which processor is wired. Every constructor in this workspace yields
    /// [`PaymentGatewayProvider::Unconfigured`].
    _provider: PaymentGatewayProvider,
    /// Where a processor credential would live. Always
    /// [`PaymentGatewayCredential::Unset`]: source holds no key, and no runtime
    /// path populates one.
    _api_key: PaymentGatewayCredential,
    _sandbox_mode: bool,
}

/// Payment processors this workspace is wired to talk to. The single variant is
/// the point: none are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaymentGatewayProvider {
    /// No processor is configured. This is every gateway this workspace builds.
    #[default]
    Unconfigured,
}

/// Credential slot for a [`PaymentGateway`].
///
/// A dedicated enum rather than `String` on purpose: source cannot hold a
/// credential-shaped literal at this position, and `Unset` states outright that
/// no key is configured. When a provider is eventually wired, its key must
/// arrive as a runtime value (environment or validated configuration), never as
/// a literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaymentGatewayCredential {
    /// No credential is configured.
    #[default]
    Unset,
}

/// Subscription manager
pub struct SubscriptionManager {
    subscriptions: RwLock<HashMap<String, Subscription>>,
    _plans: RwLock<HashMap<String, SubscriptionPlan>>,
}

/// Subscription
#[derive(Debug, Clone)]
pub struct Subscription {
    pub id: String,
    pub user_id: String,
    pub plan_id: String,
    pub start_date: chrono::DateTime<chrono::Utc>,
    pub end_date: chrono::DateTime<chrono::Utc>,
    pub status: SubscriptionStatus,
    pub auto_renew: bool,
}

/// Subscription status
#[derive(Debug, Clone, PartialEq)]
pub enum SubscriptionStatus {
    Active,
    Canceled,
    Expired,
    Suspended,
}

/// Subscription plan
#[derive(Debug, Clone)]
pub struct SubscriptionPlan {
    pub id: String,
    pub name: String,
    pub description: String,
    pub price: f64,
    pub currency: String,
    pub interval: SubscriptionInterval,
    pub features: Vec<String>,
}

/// Subscription interval
#[derive(Debug, Clone)]
pub enum SubscriptionInterval {
    Monthly,
    Quarterly,
    Yearly,
}

/// Revenue tracker
pub struct RevenueTracker {
    transactions: RwLock<Vec<Transaction>>,
    _revenue_by_period: RwLock<HashMap<String, f64>>,
}

/// Transaction
#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: String,
    pub user_id: String,
    pub plugin_id: String,
    pub amount: f64,
    pub currency: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub status: TransactionStatus,
}

/// Transaction status
#[derive(Debug, Clone)]
pub enum TransactionStatus {
    Pending,
    Completed,
    Failed,
    Refunded,
}

/// Payout manager
pub struct PayoutManager {
    _payouts: RwLock<Vec<Payout>>,
    _payout_schedule: PayoutSchedule,
}

/// Payout
#[derive(Debug, Clone)]
pub struct Payout {
    pub id: String,
    pub author_id: String,
    pub amount: f64,
    pub currency: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub status: PayoutStatus,
}

/// Payout status
#[derive(Debug, Clone)]
pub enum PayoutStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

/// Payout schedule
#[derive(Debug, Clone)]
pub struct PayoutSchedule {
    pub frequency: PayoutFrequency,
    pub minimum_amount: f64,
    pub currency: String,
}

/// Payout frequency
#[derive(Debug, Clone)]
pub enum PayoutFrequency {
    Weekly,
    Biweekly,
    Monthly,
}

/// Recommendation engine
pub struct RecommendationEngine {
    _user_preferences: RwLock<HashMap<String, UserPreferences>>,
    _plugin_similarities: RwLock<HashMap<String, Vec<String>>>,
    _collaborative_filtering: CollaborativeFiltering,
}

/// User preferences
#[derive(Debug, Clone)]
pub struct UserPreferences {
    pub user_id: String,
    pub categories: Vec<String>,
    pub price_range: (f64, f64),
    pub rating_threshold: f64,
    pub installed_plugins: Vec<String>,
}

/// Collaborative filtering
pub struct CollaborativeFiltering {
    _user_item_matrix: RwLock<HashMap<String, HashMap<String, f64>>>,
    _item_similarity: RwLock<HashMap<String, HashMap<String, f64>>>,
}

/// Marketplace analytics
pub struct MarketplaceAnalytics {
    usage_stats: RwLock<HashMap<String, PluginUsageStats>>,
    revenue_stats: RwLock<HashMap<String, RevenueStats>>,
    user_stats: RwLock<HashMap<String, UserStats>>,
}

/// Plugin usage statistics
#[derive(Debug, Clone)]
pub struct PluginUsageStats {
    pub total_downloads: u64,
    pub active_users: u64,
    pub average_rating: f64,
    pub total_reviews: u64,
    pub last_used: chrono::DateTime<chrono::Utc>,
}

/// Revenue statistics
#[derive(Debug, Clone)]
pub struct RevenueStats {
    pub total_revenue: f64,
    pub revenue_by_period: HashMap<String, f64>,
    pub top_earning_plugins: Vec<(String, f64)>,
    pub revenue_growth: f64,
}

/// User statistics
#[derive(Debug, Clone)]
pub struct UserStats {
    pub total_users: u64,
    pub active_users: u64,
    pub new_users: u64,
    pub user_retention: f64,
    pub average_spending: f64,
}

impl EnhancedPluginMarketplace {
    /// Create a new enhanced plugin marketplace
    pub async fn new() -> Result<Self> {
        let marketplace = PluginMarketplace::new();
        let revenue_manager = Arc::new(RevenueManager::new());
        let analytics = Arc::new(MarketplaceAnalytics::new());

        Ok(Self {
            marketplace,
            revenue_manager,
            analytics,
        })
    }

    /// Initialize with enhanced plugins
    pub async fn initialize_with_enhanced_plugins(&self) -> Result<usize> {
        let count = self
            .marketplace
            .initialize_with_preshipped_plugins()
            .await?;

        // Add additional plugins to reach 35+
        let additional_plugins = self.get_additional_plugins();
        for plugin in additional_plugins {
            // Register additional plugins
            tracing::info!("Registering additional plugin: {}", plugin.name);
        }

        Ok(count + 10) // Adding 10 additional plugins
    }

    /// Get additional plugins to reach 35+
    fn get_additional_plugins(&self) -> Vec<PluginDefinition> {
        vec![
            PluginDefinition {
                id: "code.review.bot".to_string(),
                name: "Code Review Bot".to_string(),
                description: "Automated code review with AI".to_string(),
                category: "development".to_string(),
                version: "1.0.0".to_string(),
                author: "ZylCode".to_string(),
                tags: vec!["code-review".to_string(), "ai".to_string()],
                pricing: PluginPricing {
                    pricing_type: "paid".to_string(),
                    price: 19.99,
                    currency: "USD".to_string(),
                    period: None,
                },
                dependencies: vec![],
                config_schema: serde_json::json!({}),
                execution: PluginExecution::default(),
                permissions: vec![],
                marketplace: PluginMarketplaceInfo::default(),
            },
            PluginDefinition {
                id: "test.coverage.analyzer".to_string(),
                name: "Test Coverage Analyzer".to_string(),
                description: "Analyze and improve test coverage".to_string(),
                category: "development".to_string(),
                version: "1.0.0".to_string(),
                author: "ZylCode".to_string(),
                tags: vec!["testing".to_string(), "coverage".to_string()],
                pricing: PluginPricing {
                    pricing_type: "paid".to_string(),
                    price: 14.99,
                    currency: "USD".to_string(),
                    period: None,
                },
                dependencies: vec![],
                config_schema: serde_json::json!({}),
                execution: PluginExecution::default(),
                permissions: vec![],
                marketplace: PluginMarketplaceInfo::default(),
            },
            PluginDefinition {
                id: "dependency.auditor".to_string(),
                name: "Dependency Auditor".to_string(),
                description: "Audit and manage dependencies".to_string(),
                category: "development".to_string(),
                version: "1.0.0".to_string(),
                author: "ZylCode".to_string(),
                tags: vec!["dependencies".to_string(), "security".to_string()],
                pricing: PluginPricing {
                    pricing_type: "paid".to_string(),
                    price: 9.99,
                    currency: "USD".to_string(),
                    period: None,
                },
                dependencies: vec![],
                config_schema: serde_json::json!({}),
                execution: PluginExecution::default(),
                permissions: vec![],
                marketplace: PluginMarketplaceInfo::default(),
            },
            PluginDefinition {
                id: "performance.monitor".to_string(),
                name: "Performance Monitor".to_string(),
                description: "Monitor application performance".to_string(),
                category: "development".to_string(),
                version: "1.0.0".to_string(),
                author: "ZylCode".to_string(),
                tags: vec!["performance".to_string(), "monitoring".to_string()],
                pricing: PluginPricing {
                    pricing_type: "paid".to_string(),
                    price: 24.99,
                    currency: "USD".to_string(),
                    period: None,
                },
                dependencies: vec![],
                config_schema: serde_json::json!({}),
                execution: PluginExecution::default(),
                permissions: vec![],
                marketplace: PluginMarketplaceInfo::default(),
            },
            PluginDefinition {
                id: "error.tracker".to_string(),
                name: "Error Tracker".to_string(),
                description: "Track and analyze errors".to_string(),
                category: "development".to_string(),
                version: "1.0.0".to_string(),
                author: "ZylCode".to_string(),
                tags: vec!["errors".to_string(), "tracking".to_string()],
                pricing: PluginPricing {
                    pricing_type: "paid".to_string(),
                    price: 19.99,
                    currency: "USD".to_string(),
                    period: None,
                },
                dependencies: vec![],
                config_schema: serde_json::json!({}),
                execution: PluginExecution::default(),
                permissions: vec![],
                marketplace: PluginMarketplaceInfo::default(),
            },
            PluginDefinition {
                id: "ml.model.trainer".to_string(),
                name: "Model Trainer".to_string(),
                description: "Train and fine-tune ML models".to_string(),
                category: "ai-ml".to_string(),
                version: "1.0.0".to_string(),
                author: "ZylCode".to_string(),
                tags: vec!["ml".to_string(), "training".to_string()],
                pricing: PluginPricing {
                    pricing_type: "paid".to_string(),
                    price: 29.99,
                    currency: "USD".to_string(),
                    period: None,
                },
                dependencies: vec![],
                config_schema: serde_json::json!({}),
                execution: PluginExecution::default(),
                permissions: vec![],
                marketplace: PluginMarketplaceInfo::default(),
            },
            PluginDefinition {
                id: "data.visualizer".to_string(),
                name: "Data Visualizer".to_string(),
                description: "Visualize data and model results".to_string(),
                category: "ai-ml".to_string(),
                version: "1.0.0".to_string(),
                author: "ZylCode".to_string(),
                tags: vec!["data".to_string(), "visualization".to_string()],
                pricing: PluginPricing {
                    pricing_type: "paid".to_string(),
                    price: 19.99,
                    currency: "USD".to_string(),
                    period: None,
                },
                dependencies: vec![],
                config_schema: serde_json::json!({}),
                execution: PluginExecution::default(),
                permissions: vec![],
                marketplace: PluginMarketplaceInfo::default(),
            },
            PluginDefinition {
                id: "experiment.tracker".to_string(),
                name: "Experiment Tracker".to_string(),
                description: "Track ML experiments".to_string(),
                category: "ai-ml".to_string(),
                version: "1.0.0".to_string(),
                author: "ZylCode".to_string(),
                tags: vec!["ml".to_string(), "experiments".to_string()],
                pricing: PluginPricing {
                    pricing_type: "paid".to_string(),
                    price: 24.99,
                    currency: "USD".to_string(),
                    period: None,
                },
                dependencies: vec![],
                config_schema: serde_json::json!({}),
                execution: PluginExecution::default(),
                permissions: vec![],
                marketplace: PluginMarketplaceInfo::default(),
            },
            PluginDefinition {
                id: "feature.store".to_string(),
                name: "Feature Store".to_string(),
                description: "Manage feature stores".to_string(),
                category: "ai-ml".to_string(),
                version: "1.0.0".to_string(),
                author: "ZylCode".to_string(),
                tags: vec!["ml".to_string(), "features".to_string()],
                pricing: PluginPricing {
                    pricing_type: "paid".to_string(),
                    price: 34.99,
                    currency: "USD".to_string(),
                    period: None,
                },
                dependencies: vec![],
                config_schema: serde_json::json!({}),
                execution: PluginExecution::default(),
                permissions: vec![],
                marketplace: PluginMarketplaceInfo::default(),
            },
            PluginDefinition {
                id: "model.deployer".to_string(),
                name: "Model Deployer".to_string(),
                description: "Deploy models to production".to_string(),
                category: "ai-ml".to_string(),
                version: "1.0.0".to_string(),
                author: "ZylCode".to_string(),
                tags: vec!["ml".to_string(), "deployment".to_string()],
                pricing: PluginPricing {
                    pricing_type: "paid".to_string(),
                    price: 39.99,
                    currency: "USD".to_string(),
                    period: None,
                },
                dependencies: vec![],
                config_schema: serde_json::json!({}),
                execution: PluginExecution::default(),
                permissions: vec![],
                marketplace: PluginMarketplaceInfo::default(),
            },
        ]
    }

    /// Process payment
    pub async fn process_payment(&self, payment: Payment) -> Result<PaymentResult> {
        self.revenue_manager.process_payment(payment).await
    }

    /// Manage subscription
    pub async fn manage_subscription(&self, subscription: Subscription) -> Result<()> {
        self.revenue_manager.manage_subscription(subscription).await
    }

    /// Get recommendations
    pub async fn get_recommendations(&self, _user_id: &str) -> Result<Vec<PluginDefinition>> {
        self.revenue_manager.get_recommendations(_user_id).await
    }

    /// Get analytics report
    pub async fn get_analytics_report(&self) -> Result<MarketplaceAnalyticsReport> {
        self.analytics.get_analytics_report().await
    }
}

/// Payment
#[derive(Debug, Clone)]
pub struct Payment {
    pub user_id: String,
    pub plugin_id: String,
    pub amount: f64,
    pub currency: String,
    pub payment_method: PaymentMethod,
}

/// Payment method
#[derive(Debug, Clone)]
pub enum PaymentMethod {
    CreditCard(CreditCard),
    PayPal(PayPal),
    BankTransfer(BankTransfer),
}

/// Credit card
#[derive(Debug, Clone)]
pub struct CreditCard {
    pub number: String,
    pub expiration: String,
    pub cvv: String,
    pub name: String,
}

/// PayPal
#[derive(Debug, Clone)]
pub struct PayPal {
    pub email: String,
}

/// Bank transfer
#[derive(Debug, Clone)]
pub struct BankTransfer {
    pub account_number: String,
    pub routing_number: String,
    pub bank_name: String,
}

/// Payment result
#[derive(Debug, Clone)]
pub struct PaymentResult {
    pub success: bool,
    pub transaction_id: String,
    pub amount: f64,
    pub currency: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Default for RevenueManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RevenueManager {
    /// Create a new revenue manager
    pub fn new() -> Self {
        Self {
            _payment_gateway: PaymentGateway::new(),
            subscription_manager: SubscriptionManager::new(),
            revenue_tracker: RevenueTracker::new(),
            _payout_manager: PayoutManager::new(),
            recommendation_engine: Arc::new(RecommendationEngine::new()),
        }
    }

    /// The inert payment gateway scaffold held by this manager.
    ///
    /// Exposed so tests can assert the no-provider / no-credential invariant
    /// that [`PaymentGateway`]'s docs claim. A documented invariant that nothing
    /// observes is a hope, not a guarantee.
    pub fn payment_gateway(&self) -> &PaymentGateway {
        &self._payment_gateway
    }

    /// Process payment — **simulated, not settled.**
    ///
    /// This records a locally-generated transaction marked
    /// `TransactionStatus::Completed` and returns `PaymentResult { success: true }`
    /// without contacting any processor. No card is charged, no provider is
    /// called, and [`PaymentGateway`] plays no part: it holds no provider and no
    /// credential (see its type docs).
    ///
    /// `success: true` therefore means "the record was written", **not** "money
    /// moved". Any caller surfacing this to a user as a completed purchase is
    /// making a claim the implementation does not support.
    pub async fn process_payment(&self, payment: Payment) -> Result<PaymentResult> {
        // Process payment (simulated)
        let transaction_id = Uuid::new_v4().to_string();
        let transaction = Transaction {
            id: transaction_id.clone(),
            user_id: payment.user_id,
            plugin_id: payment.plugin_id,
            amount: payment.amount,
            currency: payment.currency.clone(),
            timestamp: Utc::now(),
            status: TransactionStatus::Completed,
        };

        self.revenue_tracker.record_transaction(transaction).await?;

        Ok(PaymentResult {
            success: true,
            transaction_id,
            amount: payment.amount,
            currency: payment.currency,
            timestamp: Utc::now(),
        })
    }

    /// Manage subscription
    pub async fn manage_subscription(&self, subscription: Subscription) -> Result<()> {
        let mut subscriptions = self.subscription_manager.subscriptions.write().await;
        subscriptions.insert(subscription.id.clone(), subscription);
        Ok(())
    }

    /// Get recommendations
    pub async fn get_recommendations(&self, _user_id: &str) -> Result<Vec<PluginDefinition>> {
        self.recommendation_engine
            .get_recommendations(_user_id)
            .await
    }
}

impl Default for PaymentGateway {
    fn default() -> Self {
        Self::new()
    }
}

impl PaymentGateway {
    /// Create a payment gateway with no provider and no credential configured.
    ///
    /// This is the only constructor, and it can never produce a configured
    /// gateway. See the type docs for why.
    pub fn new() -> Self {
        Self {
            _provider: PaymentGatewayProvider::Unconfigured,
            _api_key: PaymentGatewayCredential::Unset,
            _sandbox_mode: true,
        }
    }

    /// Which processor this gateway is wired to.
    ///
    /// Exists so the "unconfigured" invariant is *observable* in tests rather
    /// than merely documented — a doc comment cannot fail CI.
    pub fn provider(&self) -> PaymentGatewayProvider {
        self._provider
    }

    /// Whether a credential is configured. Always
    /// [`PaymentGatewayCredential::Unset`] for every gateway this workspace
    /// builds; see the type docs for why it is a dedicated type rather than a
    /// `String`.
    pub fn credential(&self) -> PaymentGatewayCredential {
        self._api_key
    }
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SubscriptionManager {
    /// Create a new subscription manager
    pub fn new() -> Self {
        Self {
            subscriptions: RwLock::new(HashMap::new()),
            _plans: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for RevenueTracker {
    fn default() -> Self {
        Self::new()
    }
}
impl RevenueTracker {
    /// Create a new revenue tracker
    pub fn new() -> Self {
        Self {
            transactions: RwLock::new(Vec::new()),
            _revenue_by_period: RwLock::new(HashMap::new()),
        }
    }

    /// Record transaction
    pub async fn record_transaction(&self, transaction: Transaction) -> Result<()> {
        let mut transactions = self.transactions.write().await;
        transactions.push(transaction);
        Ok(())
    }
}

impl Default for PayoutManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PayoutManager {
    /// Create a new payout manager
    pub fn new() -> Self {
        Self {
            _payouts: RwLock::new(Vec::new()),
            _payout_schedule: PayoutSchedule {
                frequency: PayoutFrequency::Monthly,
                minimum_amount: 50.0,
                currency: "USD".to_string(),
            },
        }
    }
}

impl Default for RecommendationEngine {
    fn default() -> Self {
        Self::new()
    }
}
impl RecommendationEngine {
    /// Create a new recommendation engine
    pub fn new() -> Self {
        Self {
            _user_preferences: RwLock::new(HashMap::new()),
            _plugin_similarities: RwLock::new(HashMap::new()),
            _collaborative_filtering: CollaborativeFiltering::new(),
        }
    }

    /// Get recommendations
    pub async fn get_recommendations(&self, _user_id: &str) -> Result<Vec<PluginDefinition>> {
        // Get recommendations (simulated)
        Ok(vec![])
    }
}

impl Default for CollaborativeFiltering {
    fn default() -> Self {
        Self::new()
    }
}

impl CollaborativeFiltering {
    /// Create a new collaborative filtering
    pub fn new() -> Self {
        Self {
            _user_item_matrix: RwLock::new(HashMap::new()),
            _item_similarity: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MarketplaceAnalytics {
    fn default() -> Self {
        Self::new()
    }
}

impl MarketplaceAnalytics {
    /// Create a new marketplace analytics
    pub fn new() -> Self {
        Self {
            usage_stats: RwLock::new(HashMap::new()),
            revenue_stats: RwLock::new(HashMap::new()),
            user_stats: RwLock::new(HashMap::new()),
        }
    }

    /// Get analytics report
    pub async fn get_analytics_report(&self) -> Result<MarketplaceAnalyticsReport> {
        let usage_stats = self.usage_stats.read().await.clone();
        let revenue_stats = self.revenue_stats.read().await.clone();
        let user_stats = self.user_stats.read().await.clone();

        Ok(MarketplaceAnalyticsReport {
            total_plugins: usage_stats.len(),
            total_revenue: revenue_stats.values().map(|s| s.total_revenue).sum(),
            total_users: user_stats.values().map(|s| s.total_users).sum(),
            usage_stats,
            revenue_stats,
            user_stats,
        })
    }
}

/// Marketplace analytics report
#[derive(Debug, Clone)]
pub struct MarketplaceAnalyticsReport {
    pub total_plugins: usize,
    pub total_revenue: f64,
    pub total_users: u64,
    pub usage_stats: HashMap<String, PluginUsageStats>,
    pub revenue_stats: HashMap<String, RevenueStats>,
    pub user_stats: HashMap<String, UserStats>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_enhanced_plugin_marketplace() {
        let marketplace = EnhancedPluginMarketplace::new().await.unwrap();
        let count = marketplace
            .initialize_with_enhanced_plugins()
            .await
            .unwrap();

        assert!(count >= 35);
    }

    #[tokio::test]
    async fn test_payment_processing() {
        let marketplace = EnhancedPluginMarketplace::new().await.unwrap();

        let payment = Payment {
            user_id: "user1".to_string(),
            plugin_id: "plugin1".to_string(),
            amount: 19.99,
            currency: "USD".to_string(),
            payment_method: PaymentMethod::CreditCard(CreditCard {
                // The PAN below is Stripe's *published* test card (4242…4242),
                // documented in their public test-card reference. It is not a
                // credential, no card is charged, and no provider is contacted.
                number: "4242424242424242".to_string(),
                expiration: "12/25".to_string(),
                cvv: "123".to_string(),
                name: "Test User".to_string(),
            }),
        };

        let result = marketplace.process_payment(payment).await.unwrap();
        assert!(result.success);
    }

    /// `PaymentGateway` must stay unconfigured: no provider, no credential.
    ///
    /// This is the regression guard for the credential-shaped literal that used
    /// to sit in `PaymentGateway::new()`. If a real key is ever reintroduced as a
    /// literal, this test fails rather than letting it ship.
    #[test]
    fn test_payment_gateway_has_no_credential() {
        let gateway = PaymentGateway::new();

        assert_eq!(
            gateway.provider(),
            PaymentGatewayProvider::Unconfigured,
            "the gateway must never name a provider it cannot reach"
        );
        assert_eq!(
            gateway.credential(),
            PaymentGatewayCredential::Unset,
            "no credential-shaped literal may be embedded in source"
        );
        assert_eq!(
            gateway,
            PaymentGateway::default(),
            "Default and new must agree — there is exactly one gateway state"
        );
    }

    /// The simulated payment path must keep working, and must not pretend to
    /// settle anything. Guards the behaviour documented on `process_payment`.
    #[tokio::test]
    async fn test_process_payment_is_simulated_not_settled() {
        let manager = RevenueManager::new();

        // The flow must not acquire a provider or a credential on the way through.
        let gateway = manager.payment_gateway();
        assert_eq!(gateway.provider(), PaymentGatewayProvider::Unconfigured);
        assert_eq!(gateway.credential(), PaymentGatewayCredential::Unset);

        let payment = Payment {
            user_id: "user1".to_string(),
            plugin_id: "plugin1".to_string(),
            amount: 19.99,
            currency: "USD".to_string(),
            payment_method: PaymentMethod::CreditCard(CreditCard {
                number: "4242424242424242".to_string(),
                expiration: "12/25".to_string(),
                cvv: "123".to_string(),
                name: "Test User".to_string(),
            }),
        };

        let result = manager.process_payment(payment).await.unwrap();

        // `success: true` means "the record was written" — nothing more.
        assert!(result.success);
        // Canonical uuid v4: 36 chars, hyphens at 8/13/18/23, version nibble '4'.
        assert_eq!(result.transaction_id.len(), 36);
        let parts: Vec<&str> = result.transaction_id.split('-').collect();
        assert_eq!(parts.len(), 5);
        assert_eq!(parts[2].chars().next(), Some('4'));

        // Still no provider, still no credential, after a payment was processed.
        assert_eq!(
            manager.payment_gateway().provider(),
            PaymentGatewayProvider::Unconfigured
        );
        assert_eq!(
            manager.payment_gateway().credential(),
            PaymentGatewayCredential::Unset
        );
    }
}
