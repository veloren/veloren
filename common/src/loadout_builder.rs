use crate::{
    assets,
    comp::{
        item::{Item, ItemKind},
        CharacterAbility, ItemConfig, Loadout,
    },
};

/// Builder for character Loadouts, containing weapon and armour items belonging
/// to a character, along with some helper methods for loading Items and
/// ItemConfig
///
/// ```
/// use veloren_common::LoadoutBuilder;
///
/// // Build a loadout with character starter defaults and a specific sword with default sword abilities
/// let loadout = LoadoutBuilder::new()
///     .defaults()
///     .active_item(LoadoutBuilder::default_item_config_from_str(
///         Some("common.items.weapons.sword.zweihander_sword_0"),
///     ))
///     .build();
/// ```
pub struct LoadoutBuilder(Loadout);

impl LoadoutBuilder {
    #[allow(clippy::new_without_default)] // TODO: Pending review in #587
    pub fn new() -> Self {
        Self(Loadout {
            active_item: None,
            second_item: None,
            shoulder: None,
            chest: None,
            belt: None,
            hand: None,
            pants: None,
            foot: None,
            back: None,
            ring: None,
            neck: None,
            lantern: None,
            head: None,
            tabard: None,
        })
    }

    /// Set default armor items for the loadout. This may vary with game
    /// updates, but should be safe defaults for a new character.
    pub fn defaults(self) -> Self {
        self.chest(Some(assets::load_expect_cloned(
            "common.items.armor.starter.rugged_chest",
        )))
        .pants(Some(assets::load_expect_cloned(
            "common.items.armor.starter.rugged_pants",
        )))
        .foot(Some(assets::load_expect_cloned(
            "common.items.armor.starter.sandals_0",
        )))
        .lantern(Some(assets::load_expect_cloned(
            "common.items.armor.starter.lantern",
        )))
    }

    /// Get the default [ItemConfig](../comp/struct.ItemConfig.html) for a tool
    /// (weapon). This information is required for the `active` and `second`
    /// weapon items in a loadout. If some customisation to the item's
    /// abilities or their timings is desired, you should create and provide
    /// the item config directly to the [active_item](#method.active_item)
    /// method
    pub fn default_item_config_from_item(maybe_item: Option<Item>) -> Option<ItemConfig> {
        if let Some(item) = maybe_item {
            if let ItemKind::Tool(tool) = item.kind {
                let mut abilities = tool.get_abilities();
                let mut ability_drain = abilities.drain(..);

                return Some(ItemConfig {
                    item,
                    ability1: ability_drain.next(),
                    ability2: ability_drain.next(),
                    ability3: ability_drain.next(),
                    block_ability: Some(CharacterAbility::BasicBlock),
                    dodge_ability: Some(CharacterAbility::Roll),
                });
            }
        }

        None
    }

    /// Get an [Item](../comp/struct.Item.html) by its string
    /// reference by loading its asset
    pub fn item_from_str(item_ref: Option<&str>) -> Option<Item> {
        item_ref.and_then(|specifier| assets::load_cloned::<Item>(&specifier).ok())
    }

    /// Get an item's (weapon's) default
    /// [ItemConfig](../comp/struct.ItemConfig.html)
    /// by string reference. This will first attempt to load the Item, then
    /// the default abilities for that item via the
    /// [default_item_config_from_item](#method.default_item_config_from_item)
    /// function
    pub fn default_item_config_from_str(item_ref: Option<&str>) -> Option<ItemConfig> {
        Self::default_item_config_from_item(Self::item_from_str(item_ref))
    }

    pub fn active_item(mut self, item: Option<ItemConfig>) -> Self {
        self.0.active_item = item;

        self
    }

    pub fn second_item(mut self, item: Option<ItemConfig>) -> Self {
        self.0.second_item = item;

        self
    }

    pub fn shoulder(mut self, item: Option<Item>) -> Self {
        self.0.shoulder = item;
        self
    }

    pub fn chest(mut self, item: Option<Item>) -> Self {
        self.0.chest = item;
        self
    }

    pub fn belt(mut self, item: Option<Item>) -> Self {
        self.0.belt = item;
        self
    }

    pub fn hand(mut self, item: Option<Item>) -> Self {
        self.0.hand = item;
        self
    }

    pub fn pants(mut self, item: Option<Item>) -> Self {
        self.0.pants = item;
        self
    }

    pub fn foot(mut self, item: Option<Item>) -> Self {
        self.0.foot = item;
        self
    }

    pub fn back(mut self, item: Option<Item>) -> Self {
        self.0.back = item;
        self
    }

    pub fn ring(mut self, item: Option<Item>) -> Self {
        self.0.ring = item;
        self
    }

    pub fn neck(mut self, item: Option<Item>) -> Self {
        self.0.neck = item;
        self
    }

    pub fn lantern(mut self, item: Option<Item>) -> Self {
        self.0.lantern = item;
        self
    }

    pub fn head(mut self, item: Option<Item>) -> Self {
        self.0.head = item;
        self
    }

    pub fn tabard(mut self, item: Option<Item>) -> Self {
        self.0.tabard = item;
        self
    }

    pub fn build(self) -> Loadout { self.0 }
}
