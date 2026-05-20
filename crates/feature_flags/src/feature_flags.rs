// Makes the derive macro's reference to `::feature_flags::FeatureFlagValue`
// resolve when the macro is invoked inside this crate itself.
extern crate self as feature_flags;

mod flags;
mod settings;
mod store;

use std::sync::LazyLock;

use gpui::{App, Context, Global, Subscription};

pub use feature_flags_macros::EnumFeatureFlag;
pub use flags::*;
pub use settings::{FeatureFlagsSettings, generate_feature_flags_schema};
pub use store::*;

pub static XENOMORPHIC_DISABLE_STAFF: LazyLock<bool> = LazyLock::new(|| {
    std::env::var("XENOMORPHIC_DISABLE_STAFF").is_ok_and(|value| !value.is_empty() && value != "0")
});

impl Global for FeatureFlagStore {}

pub trait FeatureFlagValue:
    Sized + Clone + Eq + Default + std::fmt::Debug + Send + Sync + 'static
{
    /// Every possible value for this flag, in the order the UI should display them.
    fn all_variants() -> &'static [Self];

    /// A stable identifier for this variant used when persisting overrides.
    fn override_key(&self) -> &'static str;

    /// Human-readable label for use in the configuration UI.
    fn label(&self) -> &'static str {
        self.override_key()
    }

    /// The variant that represents "on" — what the store resolves to when
    /// staff rules, `enabled_for_all`, or a server announcement apply.
    ///
    /// For enum flags this is usually the same as [`Default::default`] (the
    /// variant marked `#[default]` in the derive). [`PresenceFlag`] overrides
    /// this so that `default() == Off` (the "unconfigured" state) but
    /// `on_variant() == On` (the "enabled" state).
    fn on_variant() -> Self {
        Self::default()
    }
}

/// Default value type for simple on/off feature flags.
///
/// The fallback value is [`PresenceFlag::Off`] so that an absent / unknown
/// flag reads as disabled; the `on_variant` override pins the "enabled"
/// state to [`PresenceFlag::On`] so staff / server / `enabled_for_all`
/// resolution still lights the flag up.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum PresenceFlag {
    On,
    #[default]
    Off,
}

/// Presence flags deref to a `bool` so call sites can use `if *flag` without
/// spelling out the enum variant — or pass them anywhere a `&bool` is wanted.
impl std::ops::Deref for PresenceFlag {
    type Target = bool;

    fn deref(&self) -> &bool {
        match self {
            PresenceFlag::On => &true,
            PresenceFlag::Off => &false,
        }
    }
}

impl FeatureFlagValue for PresenceFlag {
    fn all_variants() -> &'static [Self] {
        &[PresenceFlag::On, PresenceFlag::Off]
    }

    fn override_key(&self) -> &'static str {
        match self {
            PresenceFlag::On => "on",
            PresenceFlag::Off => "off",
        }
    }

    fn label(&self) -> &'static str {
        match self {
            PresenceFlag::On => "On",
            PresenceFlag::Off => "Off",
        }
    }

    fn on_variant() -> Self {
        PresenceFlag::On
    }
}

/// To create a feature flag, implement this trait on a trivial type and use it as
/// a generic parameter when called [`FeatureFlagAppExt::has_flag`].
///
/// Feature flags are enabled for members of Xenomorphic staff by default. To disable this behavior
/// so you can test flags being disabled, set XENOMORPHIC_DISABLE_STAFF=1 in your environment,
/// which will force Xenomorphic to treat the current user as non-staff.
pub trait FeatureFlag {
    const NAME: &'static str;

    /// The type of value this flag can hold. Use [`PresenceFlag`] for simple
    /// on/off flags.
    type Value: FeatureFlagValue;

    /// Returns whether this feature flag is enabled for Xenomorphic staff.
    fn enabled_for_staff() -> bool {
        true
    }

    /// Returns whether this feature flag is enabled for everyone.
    ///
    /// This is generally done on the server, but we provide this as a way to entirely enable a feature flag client-side
    /// without needing to remove all of the call sites.
    fn enabled_for_all() -> bool {
        false
    }

    /// Subscribes the current view to changes in the feature flag store, so
    /// that any mutation of flags or overrides will trigger a re-render.
    ///
    /// The returned subscription is immediately detached; use [`observe_flag`]
    /// directly if you need to hold onto the subscription.
    fn watch<V: 'static>(cx: &mut Context<V>) {
        cx.observe_global::<FeatureFlagStore>(|_, cx| cx.notify())
            .detach();
    }
}

pub trait FeatureFlagAppExt {
    fn update_flags(&mut self, staff: bool, flags: Vec<String>);
    fn set_staff(&mut self, staff: bool);
    fn has_flag<T: FeatureFlag>(&self) -> bool;
    fn flag_value<T: FeatureFlag>(&self) -> T::Value;
    fn is_staff(&self) -> bool;

    fn observe_flag<T: FeatureFlag, F>(&mut self, callback: F) -> Subscription
    where
        F: FnMut(T::Value, &mut App) + 'static;
}

impl FeatureFlagAppExt for App {
    fn update_flags(&mut self, staff: bool, _flags: Vec<String>) {
        // Cloud-delivered flags are no longer supported.
        // This method is retained for test compatibility — it only sets staff status.
        let store = self.default_global::<FeatureFlagStore>();
        store.set_staff(staff);
    }

    fn set_staff(&mut self, staff: bool) {
        let store = self.default_global::<FeatureFlagStore>();
        store.set_staff(staff);
    }

    fn has_flag<T: FeatureFlag>(&self) -> bool {
        self.try_global::<FeatureFlagStore>()
            .map(|store| store.has_flag::<T>(self))
            .unwrap_or_else(|| FeatureFlagStore::has_flag_default::<T>())
    }

    fn flag_value<T: FeatureFlag>(&self) -> T::Value {
        self.try_global::<FeatureFlagStore>()
            .and_then(|store| store.try_flag_value::<T>(self))
            .unwrap_or_default()
    }

    fn is_staff(&self) -> bool {
        self.try_global::<FeatureFlagStore>()
            .map(|store| store.is_staff())
            .unwrap_or(false)
    }

    fn observe_flag<T: FeatureFlag, F>(&mut self, mut callback: F) -> Subscription
    where
        F: FnMut(T::Value, &mut App) + 'static,
    {
        let mut last_value: Option<T::Value> = None;
        self.observe_global::<FeatureFlagStore>(move |cx| {
            let value = cx.flag_value::<T>();
            if last_value.as_ref() == Some(&value) {
                return;
            }
            last_value = Some(value.clone());
            callback(value, cx);
        })
    }
}
