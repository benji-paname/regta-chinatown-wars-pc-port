// SPDX-License-Identifier: GPL-3.0-only
//! Grand Theft Auto: Chinatown Wars authentic PDA Engine and Apps.
//! Implements the authentic PDA systems matching PSP overlays:
//! - `pdaapp` (Overlay 096, 62 KB): PDA home screen, notifications, battery/time.
//! - `emailapp` (Overlay 023, 35 KB): Story emails, contact messages, and market tip push alerts.
//! - `ammozoneapp` (Overlay 001, 42 KB): Ammu-Nation online web store with safehouse delivery.
//! - `tradestatsapp` (Overlay 149, 47 KB): Commodity price trends, historical profit, and market insights.

#![allow(dead_code)]

use crate::drugs::{DrugType, Gang, MarketTip};

/// PDA Application currently displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdaView {
    Home,
    EmailInbox,
    EmailDetail(usize),
    AmmuNationStore,
    TradeStats,
    SystemSettings,
}

/// An incoming email message in Huang's PDA inbox.
#[derive(Debug, Clone, PartialEq)]
pub struct EmailMessage {
    pub id: u32,
    pub sender: &'static str,
    pub subject: &'static str,
    pub body: &'static str,
    pub read: bool,
    pub timestamp_str: &'static str,
    pub market_tip: Option<MarketTip>,
    pub waypoint: Option<[f32; 3]>,
}

/// A purchasable weapon or gear item in the Ammu-Nation PDA store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreItem {
    pub id: usize,
    pub type_id: usize, // Matches WeaponRecord type_id
    pub name: &'static str,
    pub price: u32,
    pub rounds_per_purchase: u16,
    pub category: &'static str,
}

pub const AMMUNATION_CATALOG: &[StoreItem] = &[
    StoreItem { id: 0, type_id: 1, name: "Batte de baseball", price: 150, rounds_per_purchase: 1, category: "Corps à corps" },
    StoreItem { id: 1, type_id: 2, name: "Pistolet 9mm", price: 500, rounds_per_purchase: 50, category: "Pistolets" },
    StoreItem { id: 2, type_id: 4, name: "Fusil à pompe", price: 1200, rounds_per_purchase: 30, category: "Fusils" },
    StoreItem { id: 3, type_id: 6, name: "SMG Micro", price: 2000, rounds_per_purchase: 120, category: "Mitraillettes" },
    StoreItem { id: 4, type_id: 7, name: "Fusil d'assaut AK-47", price: 3500, rounds_per_purchase: 150, category: "Fusils d'assaut" },
    StoreItem { id: 5, type_id: 10, name: "Lance-roquettes RPG", price: 10000, rounds_per_purchase: 8, category: "Armes lourdes" },
    StoreItem { id: 6, type_id: 13, name: "Grenades à fragmentation", price: 800, rounds_per_purchase: 10, category: "Projectiles" },
    StoreItem { id: 7, type_id: 99, name: "Gilet pare-balles (Kevlar)", price: 500, rounds_per_purchase: 100, category: "Protection" },
];

/// The complete PDA system of Huang Lee.
pub struct PdaEngine {
    pub open: bool,
    pub current_view: PdaView,
    pub emails: Vec<EmailMessage>,
    pub selected_index: usize,
    pub unread_count: usize,
}

impl Default for PdaEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PdaEngine {
    pub fn new() -> Self {
        let initial_emails = vec![
            EmailMessage {
                id: 1,
                sender: "Uncle Kenny",
                subject: "Bienvenue à Liberty City, Huang",
                body: "Huang, mon neveu. La perte de ton père nous frappe tous. Mais l'épée Yu Jian a été dérobée à ton arrivée. Rejoins mon restaurant à Cerveza Heights immédiatement pour tirer cette affaire au clair.",
                read: false,
                timestamp_str: "10:15",
                market_tip: None,
                waypoint: Some([485.0, 275.0, 6.0]),
            },
            EmailMessage {
                id: 2,
                sender: "Info Deal : Alonzo",
                subject: "Pénurie d'Acide chez les Yardies !",
                body: "Yo Huang, les Yardies à Meadow Hills sont à sec d'Acide. Ils payent le prix fort (plus de 400$ l'unité). Si t'en as, fonce leur en vendre !",
                read: false,
                timestamp_str: "10:42",
                market_tip: Some(MarketTip::Shortage {
                    gang: Gang::Yardies,
                    drug: DrugType::Acid,
                    offered_price: 420,
                    district: "Meadow Hills",
                }),
                waypoint: Some([710.0, 340.0, 5.0]),
            },
            EmailMessage {
                id: 3,
                sender: "Ammu-Nation Web Store",
                subject: "Votre catalogue d'armes personnel",
                body: "Ammu-Nation vous souhaite la bienvenue dans la liberté américaine ! Commandez vos armes de poing, fusils et protections par notre boutique en ligne. Livraison directe et discrète dans votre planque.",
                read: false,
                timestamp_str: "11:00",
                market_tip: None,
                waypoint: None,
            },
        ];

        Self {
            open: false,
            current_view: PdaView::Home,
            emails: initial_emails,
            selected_index: 0,
            unread_count: 3,
        }
    }

    /// Toggles PDA screen on or off.
    pub fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            self.current_view = PdaView::Home;
            self.selected_index = 0;
        }
    }

    /// Receives a new email notification.
    pub fn receive_email(&mut self, email: EmailMessage) {
        self.emails.insert(0, email);
        self.unread_count += 1;
    }

    /// Marks the selected email as read.
    pub fn open_selected_email(&mut self) -> Option<&EmailMessage> {
        if self.selected_index < self.emails.len() {
            let email = &mut self.emails[self.selected_index];
            if !email.read {
                email.read = true;
                self.unread_count = self.unread_count.saturating_sub(1);
            }
            self.current_view = PdaView::EmailDetail(self.selected_index);
            Some(&self.emails[self.selected_index])
        } else {
            None
        }
    }

    /// Navigates selection up or down.
    pub fn navigate(&mut self, delta: i32) {
        let max = match self.current_view {
            PdaView::Home => 4, // 4 main apps (Email, Ammu-Nation, Trade Stats, Settings)
            PdaView::EmailInbox => self.emails.len().saturating_sub(1),
            PdaView::AmmuNationStore => AMMUNATION_CATALOG.len().saturating_sub(1),
            _ => 0,
        };
        if max == 0 {
            return;
        }
        let current = self.selected_index as i32;
        let new_idx = (current + delta).clamp(0, max as i32) as usize;
        self.selected_index = new_idx;
    }

    /// Purchase an item from the Ammu-Nation web store.
    pub fn purchase_store_item(
        &self,
        catalog_idx: usize,
        player_cash: &mut u32,
        weapons: Option<&mut crate::weapons::WeaponManager>,
        armour: &mut u8,
    ) -> Result<&'static str, &'static str> {
        let item = AMMUNATION_CATALOG.get(catalog_idx).ok_or("Article introuvable")?;
        if *player_cash < item.price {
            return Err("Fonds insuffisants");
        }

        *player_cash -= item.price;
        if item.type_id == 99 {
            // Armour purchase
            *armour = 127;
        } else if let Some(wm) = weapons {
            wm.give(item.type_id as u8, &crate::weapons::WeaponTables::default());
        }

        Ok(item.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pda_initial_state_and_unread_emails() {
        let mut pda = PdaEngine::new();
        assert!(!pda.open);
        assert_eq!(pda.unread_count, 3);
        assert_eq!(pda.emails.len(), 3);

        // Open PDA and inspect first email
        pda.toggle();
        assert!(pda.open);
        assert_eq!(pda.current_view, PdaView::Home);

        // Open Email inbox
        pda.current_view = PdaView::EmailInbox;
        let email = pda.open_selected_email().unwrap();
        assert_eq!(email.sender, "Uncle Kenny");
        assert!(email.read);
        assert_eq!(pda.unread_count, 2);
    }

    #[test]
    fn pda_navigation_and_clamping() {
        let mut pda = PdaEngine::new();
        pda.current_view = PdaView::EmailInbox;
        pda.navigate(1);
        assert_eq!(pda.selected_index, 1);
        pda.navigate(10);
        assert_eq!(pda.selected_index, 2); // Clamped to max emails (3 items -> index 2)
        pda.navigate(-10);
        assert_eq!(pda.selected_index, 0);
    }

    #[test]
    fn ammunation_store_purchase_and_cash_deduction() {
        let pda = PdaEngine::new();
        let mut cash = 1000u32;
        let mut armour = 0u8;

        // Buy Pistol ($500)
        let res = pda.purchase_store_item(1, &mut cash, None, &mut armour);
        assert!(res.is_ok());
        assert_eq!(cash, 500);

        // Buy Kevlar Armor ($500)
        let res_armour = pda.purchase_store_item(7, &mut cash, None, &mut armour);
        assert!(res_armour.is_ok());
        assert_eq!(cash, 0);
        assert_eq!(armour, 127);

        // Insufficient cash
        let res_fail = pda.purchase_store_item(0, &mut cash, None, &mut armour);
        assert!(res_fail.is_err());
    }
}
