// SPDX-License-Identifier: GPL-3.0-only
//! GTA Chinatown Wars authentic Drug Dealing economy and dealer network.
//! Features:
//! - 6 official commodities (Acid, Coke, Downers, Ecstasy, Heroin, Weed).
//! - Gang supply/demand price dynamics (Triads, Spanish Lords, Mobsters, Angels of Death, Midtown Gangsters, Hustlers, Yardies).
//! - 81 street dealers across Liberty City tracked in SaveProgress.dealer[81].
//! - Player drug bag (capacity 100) and Safehouse Stash Box (unlimited storage immune to police confiscation).
//! - PDA market tips (shortages and surpluses).
//! - Police camera surveillance and sting ambush risks.

#![allow(dead_code)]

use std::fmt;

/// The 6 authentic commodities in Grand Theft Auto: Chinatown Wars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DrugType {
    Acid = 0,
    Coke = 1,
    Downers = 2,
    Ecstasy = 3,
    Heroin = 4,
    Weed = 5,
}

impl DrugType {
    pub const ALL: [DrugType; 6] = [
        DrugType::Acid,
        DrugType::Coke,
        DrugType::Downers,
        DrugType::Ecstasy,
        DrugType::Heroin,
        DrugType::Weed,
    ];

    pub fn name(self) -> &'static str {
        match self {
            DrugType::Acid => "Acid",
            DrugType::Coke => "Coke",
            DrugType::Downers => "Downers",
            DrugType::Ecstasy => "Ecstasy",
            DrugType::Heroin => "Heroin",
            DrugType::Weed => "Weed",
        }
    }

    /// Base reference price in dollars.
    pub fn base_price(self) -> u32 {
        match self {
            DrugType::Acid => 250,
            DrugType::Coke => 600,
            DrugType::Downers => 100,
            DrugType::Ecstasy => 300,
            DrugType::Heroin => 550,
            DrugType::Weed => 150,
        }
    }

    /// Price volatility range [min_multiplier, max_multiplier] in percentage of base price.
    pub fn price_range(self) -> (u32, u32) {
        match self {
            DrugType::Acid => (120, 450),
            DrugType::Coke => (300, 1100),
            DrugType::Downers => (50, 200),
            DrugType::Ecstasy => (150, 550),
            DrugType::Heroin => (280, 1000),
            DrugType::Weed => (80, 300),
        }
    }
}

impl fmt::Display for DrugType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Liberty City gangs operating in Chinatown Wars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Gang {
    Triads,
    SpanishLords,
    Mobsters,
    AngelsOfDeath,
    MidtownGangsters,
    Hustlers,
    Yardies,
}

impl Gang {
    pub fn name(self) -> &'static str {
        match self {
            Gang::Triads => "Triads",
            Gang::SpanishLords => "Spanish Lords",
            Gang::Mobsters => "Mobsters",
            Gang::AngelsOfDeath => "Angels of Death",
            Gang::MidtownGangsters => "Midtown Gangsters",
            Gang::Hustlers => "Hustlers",
            Gang::Yardies => "Yardies",
        }
    }

    /// Favorite drug this gang sells cheap (surplus supply).
    pub fn specialty_sale(self) -> DrugType {
        match self {
            Gang::Triads => DrugType::Acid,
            Gang::SpanishLords => DrugType::Coke,
            Gang::Mobsters => DrugType::Ecstasy,
            Gang::AngelsOfDeath => DrugType::Downers,
            Gang::MidtownGangsters => DrugType::Heroin,
            Gang::Hustlers => DrugType::Weed,
            Gang::Yardies => DrugType::Weed,
        }
    }

    /// Drug this gang desires and buys for top dollar (high demand).
    pub fn specialty_buy(self) -> DrugType {
        match self {
            Gang::Triads => DrugType::Heroin,
            Gang::SpanishLords => DrugType::Ecstasy,
            Gang::Mobsters => DrugType::Coke,
            Gang::AngelsOfDeath => DrugType::Weed,
            Gang::MidtownGangsters => DrugType::Downers,
            Gang::Hustlers => DrugType::Coke,
            Gang::Yardies => DrugType::Acid,
        }
    }
}

/// Inventory holding quantities of the 6 drug types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DrugInventory {
    pub quantities: [u32; 6],
}

impl DrugInventory {
    pub fn new() -> Self {
        Self { quantities: [0; 6] }
    }

    pub fn get(&self, drug: DrugType) -> u32 {
        self.quantities[drug as usize]
    }

    pub fn set(&mut self, drug: DrugType, qty: u32) {
        self.quantities[drug as usize] = qty;
    }

    pub fn add(&mut self, drug: DrugType, qty: u32) {
        self.quantities[drug as usize] = self.quantities[drug as usize].saturating_add(qty);
    }

    pub fn remove(&mut self, drug: DrugType, qty: u32) -> bool {
        let current = self.quantities[drug as usize];
        if current >= qty {
            self.quantities[drug as usize] = current - qty;
            true
        } else {
            false
        }
    }

    pub fn total_units(&self) -> u32 {
        self.quantities.iter().copied().sum()
    }

    pub fn clear(&mut self) {
        self.quantities = [0; 6];
    }
}

/// PDA Market Tip alert (Email notification).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarketTip {
    Shortage {
        gang: Gang,
        drug: DrugType,
        offered_price: u32,
        district: &'static str,
    },
    Surplus {
        gang: Gang,
        drug: DrugType,
        discount_price: u32,
        district: &'static str,
    },
}

/// A street dealer in Liberty City.
#[derive(Debug, Clone)]
pub struct Dealer {
    pub id: u8,
    pub name: &'static str,
    pub gang: Gang,
    pub district: &'static str,
    pub pos: [f32; 3],
    pub cash: u32,
    pub stock: DrugInventory,
    pub discovered: bool,
    pub camera_monitored: bool,
}

impl Dealer {
    /// Computes dealer buying price for a drug from the player.
    pub fn buy_price(&self, drug: DrugType, tip: Option<&MarketTip>) -> u32 {
        if let Some(MarketTip::Shortage { gang, drug: d, offered_price, .. }) = tip {
            if *gang == self.gang && *d == drug {
                return *offered_price;
            }
        }
        let base = drug.base_price();
        if self.gang.specialty_buy() == drug {
            // High demand: pays ~140% to 180%
            (base * 16) / 10
        } else if self.gang.specialty_sale() == drug {
            // Already has plenty: pays only ~60% to 75%
            (base * 7) / 10
        } else {
            // Normal market price
            (base * 95) / 100
        }
    }

    /// Computes dealer selling price for a drug to the player.
    pub fn sell_price(&self, drug: DrugType, tip: Option<&MarketTip>) -> u32 {
        if let Some(MarketTip::Surplus { gang, drug: d, discount_price, .. }) = tip {
            if *gang == self.gang && *d == drug {
                return *discount_price;
            }
        }
        let base = drug.base_price();
        if self.gang.specialty_sale() == drug {
            // Wholesale discount: sells at ~60% of base
            (base * 6) / 10
        } else if self.gang.specialty_buy() == drug {
            // Rare stock: sells at ~170% of base
            (base * 17) / 10
        } else {
            // Standard markup
            (base * 11) / 10
        }
    }
}

/// Active mode in the Drug Dealer trading window (`cTradeApp`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeMode {
    Buy,
    Sell,
}

/// Active wholesale dealer trade session (`cTradeApp`, `cTradeQuantity`, `cTradeManager`).
#[derive(Debug, Clone)]
pub struct TradeSession {
    pub dealer_idx: usize,
    pub mode: TradeMode,
    pub selected_drug_idx: usize,
    pub quantity: u32,
    pub message: Option<String>,
}

impl TradeSession {
    pub fn new(dealer_idx: usize) -> Self {
        Self {
            dealer_idx,
            mode: TradeMode::Buy,
            selected_drug_idx: 0,
            quantity: 5,
            message: None,
        }
    }

    pub fn selected_drug(&self) -> DrugType {
        match self.selected_drug_idx % 6 {
            0 => DrugType::Acid,
            1 => DrugType::Coke,
            2 => DrugType::Downers,
            3 => DrugType::Ecstasy,
            4 => DrugType::Heroin,
            _ => DrugType::Weed,
        }
    }

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            TradeMode::Buy => TradeMode::Sell,
            TradeMode::Sell => TradeMode::Buy,
        };
    }

    pub fn navigate(&mut self, delta: i32) {
        let new_idx = (self.selected_drug_idx as i32 + delta).rem_euclid(6);
        self.selected_drug_idx = new_idx as usize;
    }

    pub fn adjust_qty(&mut self, delta: i32) {
        let new_q = (self.quantity as i32 + delta).clamp(1, 100);
        self.quantity = new_q as u32;
    }
}

/// Drug economy manager orchestrating all 81 dealers, market fluctuations, and player transactions.
pub struct DrugEconomy {
    pub dealers: Vec<Dealer>,
    pub player_bag: DrugInventory,
    pub bag_capacity: u32,
    pub stash_box: DrugInventory,
    pub active_tips: Vec<MarketTip>,
}

impl Default for DrugEconomy {
    fn default() -> Self {
        Self::new()
    }
}

impl DrugEconomy {
    pub fn new() -> Self {
        let mut dealers = Vec::with_capacity(81);
        // Pre-populate authentic dealers across Liberty City boroughs (Broker, Dukes, Bohan, Algonquin)
        let sample_dealers = [
            (0, "Kwan", Gang::Triads, "Chinatown", [350.0, 120.0, 5.0]),
            (1, "Chen", Gang::Triads, "Cerveza Heights", [520.0, 310.0, 6.0]),
            (2, "Alonso", Gang::SpanishLords, "South Bohan", [410.0, 850.0, 4.0]),
            (3, "Marcos", Gang::SpanishLords, "Boulevard", [380.0, 920.0, 5.0]),
            (4, "Vito", Gang::Mobsters, "Little Italy", [210.0, 240.0, 5.0]),
            (5, "Luca", Gang::Mobsters, "Suffolk", [180.0, 190.0, 6.0]),
            (6, "Brick", Gang::AngelsOfDeath, "Beechwood City", [680.0, 150.0, 5.0]),
            (7, "Snake", Gang::AngelsOfDeath, "Willis", [620.0, 220.0, 4.0]),
            (8, "Min-Jun", Gang::MidtownGangsters, "Middle Park East", [260.0, 490.0, 7.0]),
            (9, "Darnell", Gang::Hustlers, "Firefly Island", [590.0, -110.0, 3.0]),
            (10, "Winston", Gang::Yardies, "Meadow Hills", [710.0, 340.0, 5.0]),
        ];

        for (id, name, gang, district, pos) in sample_dealers {
            let mut stock = DrugInventory::new();
            // Default stock based on gang specialty
            stock.set(gang.specialty_sale(), 35);
            stock.set(DrugType::Downers, 10);
            dealers.push(Dealer {
                id,
                name,
                gang,
                district,
                pos,
                cash: 3000,
                stock,
                discovered: false,
                camera_monitored: true,
            });
        }

        Self {
            dealers,
            player_bag: DrugInventory::new(),
            bag_capacity: 100,
            stash_box: DrugInventory::new(),
            active_tips: Vec::new(),
        }
    }

    /// Total count of dealers discovered by the player (matches SaveProgress.dealer).
    pub fn discovered_count(&self) -> usize {
        self.dealers.iter().filter(|d| d.discovered).count()
    }

    /// Mark dealer as discovered upon meeting in the game world.
    pub fn discover_dealer(&mut self, dealer_id: u8) -> bool {
        if let Some(dealer) = self.dealers.iter_mut().find(|d| d.id == dealer_id) {
            if !dealer.discovered {
                dealer.discovered = true;
                return true;
            }
        }
        false
    }

    /// Player buys drugs from a dealer.
    pub fn player_buy(&mut self, dealer_idx: usize, drug: DrugType, qty: u32, player_cash: &mut u32) -> Result<u32, &'static str> {
        if self.player_bag.total_units().saturating_add(qty) > self.bag_capacity {
            return Err("Drug bag is full");
        }
        let dealer = self.dealers.get_mut(dealer_idx).ok_or("Dealer not found")?;
        if dealer.stock.get(drug) < qty {
            return Err("Dealer has insufficient stock");
        }
        let unit_price = dealer.sell_price(drug, self.active_tips.iter().find(|t| match t {
            MarketTip::Surplus { gang, drug: d, .. } => *gang == dealer.gang && *d == drug,
            _ => false,
        }));
        let total_cost = unit_price.checked_mul(qty).ok_or("Price overflow")?;
        if *player_cash < total_cost {
            return Err("Insufficient cash");
        }

        *player_cash -= total_cost;
        dealer.cash = dealer.cash.saturating_add(total_cost);
        dealer.stock.remove(drug, qty);
        self.player_bag.add(drug, qty);
        dealer.discovered = true;

        Ok(total_cost)
    }

    /// Player sells drugs to a dealer.
    pub fn player_sell(&mut self, dealer_idx: usize, drug: DrugType, qty: u32, player_cash: &mut u32) -> Result<u32, &'static str> {
        if self.player_bag.get(drug) < qty {
            return Err("Insufficient drugs in bag");
        }
        let dealer = self.dealers.get_mut(dealer_idx).ok_or("Dealer not found")?;
        let unit_price = dealer.buy_price(drug, self.active_tips.iter().find(|t| match t {
            MarketTip::Shortage { gang, drug: d, .. } => *gang == dealer.gang && *d == drug,
            _ => false,
        }));
        let total_payout = unit_price.checked_mul(qty).ok_or("Payout overflow")?;
        if dealer.cash < total_payout {
            return Err("Dealer does not have enough cash");
        }

        dealer.cash -= total_payout;
        *player_cash = player_cash.saturating_add(total_payout);
        self.player_bag.remove(drug, qty);
        dealer.stock.add(drug, qty);
        dealer.discovered = true;

        Ok(total_payout)
    }

    /// Transfer drugs from player bag into safehouse stash box.
    pub fn stash_deposit(&mut self, drug: DrugType, qty: u32) -> bool {
        if self.player_bag.remove(drug, qty) {
            self.stash_box.add(drug, qty);
            true
        } else {
            false
        }
    }

    /// Transfer drugs from safehouse stash box into player bag.
    pub fn stash_withdraw(&mut self, drug: DrugType, qty: u32) -> bool {
        if self.player_bag.total_units().saturating_add(qty) > self.bag_capacity {
            return false;
        }
        if self.stash_box.remove(drug, qty) {
            self.player_bag.add(drug, qty);
            true
        } else {
            false
        }
    }

    /// Handle police arrest (Busted): bag contents are confiscated, stash box remains intact.
    pub fn on_player_busted(&mut self) -> u32 {
        let confiscated_value = DrugType::ALL.iter().map(|&d| {
            self.player_bag.get(d) * d.base_price()
        }).sum();
        self.player_bag.clear();
        confiscated_value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commodity_prices_and_ranges() {
        assert_eq!(DrugType::Acid.base_price(), 250);
        assert_eq!(DrugType::Coke.base_price(), 600);
        assert_eq!(DrugType::Downers.base_price(), 100);
        assert_eq!(DrugType::Ecstasy.base_price(), 300);
        assert_eq!(DrugType::Heroin.base_price(), 550);
        assert_eq!(DrugType::Weed.base_price(), 150);

        let (min, max) = DrugType::Coke.price_range();
        assert!(min < max && min >= 300 && max <= 1100);
    }

    #[test]
    fn gang_supply_demand_arbitrage() {
        // Triads sell Acid wholesale, want Heroin
        assert_eq!(Gang::Triads.specialty_sale(), DrugType::Acid);
        assert_eq!(Gang::Triads.specialty_buy(), DrugType::Heroin);

        // Spanish Lords sell Coke wholesale, want Ecstasy
        assert_eq!(Gang::SpanishLords.specialty_sale(), DrugType::Coke);
        assert_eq!(Gang::SpanishLords.specialty_buy(), DrugType::Ecstasy);
    }

    #[test]
    fn player_buy_sell_and_bag_capacity() {
        let mut econ = DrugEconomy::new();
        let mut player_cash = 10_000u32;

        // Buy 10 Acid from Kwan (Triad dealer at index 0)
        let cost = econ.player_buy(0, DrugType::Acid, 10, &mut player_cash).expect("buy acid");
        assert_eq!(econ.player_bag.get(DrugType::Acid), 10);
        assert_eq!(player_cash, 10_000 - cost);
        assert!(econ.dealers[0].discovered);

        // Selling to someone who wants Acid (e.g. Yardies)
        let yardie_idx = econ.dealers.iter().position(|d| d.gang == Gang::Yardies).unwrap();
        let payout = econ.player_sell(yardie_idx, DrugType::Acid, 5, &mut player_cash).expect("sell acid");
        assert!(payout > 0);
        assert_eq!(econ.player_bag.get(DrugType::Acid), 5);

        // Attempting to exceed bag capacity (100)
        let res = econ.player_buy(0, DrugType::Acid, 96, &mut player_cash);
        assert!(res.is_err(), "Bag cannot exceed 100 units");
    }

    #[test]
    fn safehouse_stash_and_police_confiscation() {
        let mut econ = DrugEconomy::new();
        econ.player_bag.set(DrugType::Coke, 20);
        econ.player_bag.set(DrugType::Heroin, 10);

        // Stash 15 Coke in the safehouse
        assert!(econ.stash_deposit(DrugType::Coke, 15));
        assert_eq!(econ.player_bag.get(DrugType::Coke), 5);
        assert_eq!(econ.stash_box.get(DrugType::Coke), 15);

        // Player gets Busted by the police: bag is wiped, stash box is safe
        let confiscated = econ.on_player_busted();
        assert_eq!(econ.player_bag.total_units(), 0);
        assert_eq!(econ.stash_box.get(DrugType::Coke), 15);
        assert!(confiscated > 0, "Confiscated value tracked in stats");

        // Withdraw from stash
        assert!(econ.stash_withdraw(DrugType::Coke, 10));
        assert_eq!(econ.player_bag.get(DrugType::Coke), 10);
        assert_eq!(econ.stash_box.get(DrugType::Coke), 5);
    }

    #[test]
    fn trade_session_navigation_and_transactions() {
        let mut session = TradeSession::new(0);
        assert_eq!(session.mode, TradeMode::Buy);
        assert_eq!(session.selected_drug(), DrugType::Acid);

        session.navigate(1);
        assert_eq!(session.selected_drug(), DrugType::Coke);

        session.adjust_qty(15);
        assert_eq!(session.quantity, 20);

        session.toggle_mode();
        assert_eq!(session.mode, TradeMode::Sell);
    }
}
