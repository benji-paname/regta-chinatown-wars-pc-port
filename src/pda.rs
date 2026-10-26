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
    ContactsList,
    ContactDetail(usize),
    AmmuNationStore,
    TradeStats,
    SystemSettings,
}

/// An entry in Huang's PDA Address Book / Contacts list (Overlay 014 contactsapp).
#[derive(Debug, Clone, PartialEq)]
pub struct PdaContact {
    pub id: usize,
    pub name: &'static str,
    pub title: &'static str,
    pub organization: &'static str,
    pub district: &'static str,
    pub phone: &'static str,
    pub meeting_pos: [f32; 3],
    pub unlocked: bool,
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
    pub contacts: Vec<PdaContact>,
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
        let initial_contacts = vec![
            PdaContact {
                id: 0,
                name: "Uncle Kenny",
                title: "Sous-chef des Triades",
                organization: "Triades de Liberty City",
                district: "Cerveza Heights",
                phone: "555-0182",
                meeting_pos: [485.0, 275.0, 6.0],
                unlocked: true,
            },
            PdaContact {
                id: 1,
                name: "Chan Jaoming",
                title: "Fils du Grand Patriarche",
                organization: "Triades de Liberty City",
                district: "Easton",
                phone: "555-0144",
                meeting_pos: [230.0, 260.0, 5.0],
                unlocked: true,
            },
            PdaContact {
                id: 2,
                name: "Zhou Ming",
                title: "Chef de faction Triade",
                organization: "Triades de Liberty City",
                district: "Middle Park",
                phone: "555-0199",
                meeting_pos: [270.0, 510.0, 7.0],
                unlocked: true,
            },
            PdaContact {
                id: 3,
                name: "Hsin Jaoming",
                title: "Grand Patriarche",
                organization: "Haut Conseil des Triades",
                district: "Chinatown",
                phone: "555-0100",
                meeting_pos: [290.0, 80.0, 5.0],
                unlocked: false,
            },
            PdaContact {
                id: 4,
                name: "Wade Heston",
                title: "Inspecteur (Affaires Internes)",
                organization: "LCPD (Police de Liberty City)",
                district: "Northwood",
                phone: "555-0155",
                meeting_pos: [160.0, 690.0, 5.0],
                unlocked: true,
            },
            PdaContact {
                id: 5,
                name: "Lester Leroc",
                title: "Agent infiltré",
                organization: "Angels of Death",
                district: "Willis",
                phone: "555-0122",
                meeting_pos: [650.0, 190.0, 4.0],
                unlocked: false,
            },
            PdaContact {
                id: 6,
                name: "Rudy D'Avanzo",
                title: "Affilié Mafia Italienne",
                organization: "Famille Messina",
                district: "Little Italy",
                phone: "555-0177",
                meeting_pos: [200.0, 230.0, 5.0],
                unlocked: false,
            },
            PdaContact {
                id: 7,
                name: "Xin Shan",
                title: "Frère de Ling (Secret)",
                organization: "Indépendant (Exclusif PSP)",
                district: "Cerveza Heights",
                phone: "555-0133",
                meeting_pos: [490.0, 290.0, 6.0],
                unlocked: false,
            },
        ];

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
            contacts: initial_contacts,
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

    /// Selects a contact from the address book and retrieves GPS coordinates (Overlay 014 contactsapp).
    pub fn select_contact_and_get_gps(&mut self, contact_idx: usize) -> Option<([f32; 3], &'static str)> {
        if contact_idx < self.contacts.len() {
            let contact = &self.contacts[contact_idx];
            self.current_view = PdaView::ContactDetail(contact_idx);
            Some((contact.meeting_pos, contact.name))
        } else {
            None
        }
    }

    /// Navigates selection up or down.
    pub fn navigate(&mut self, delta: i32) {
        let max = match self.current_view {
            PdaView::Home => 5, // 5 main apps (Email, Contacts, Ammu-Nation, Trade Stats, Settings)
            PdaView::EmailInbox => self.emails.len().saturating_sub(1),
            PdaView::ContactsList => self.contacts.len().saturating_sub(1),
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

// ============================================================================
// Authentic Chinatown Wars PDA & HUD Classes (cWnd, cContainerWnd, cListBox, cIPhonePad, cContextHelp)
// ============================================================================

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub struct Wnd {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub title: &'static str,
    pub visible: bool,
    pub focused: bool,
    pub bg_color: [f32; 4],
    pub border_color: [f32; 4],
}

#[allow(dead_code)]
impl Wnd {
    pub fn new(x: f32, y: f32, w: f32, h: f32, title: &'static str) -> Self {
        Self {
            x,
            y,
            w,
            h,
            title,
            visible: true,
            focused: false,
            bg_color: [0.08, 0.09, 0.11, 0.92],
            border_color: [0.95, 0.78, 0.22, 1.0],
        }
    }

    pub fn set_bounds(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.x = x;
        self.y = y;
        self.w = w;
        self.h = h;
    }

    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn bounds(&self) -> [f32; 4] {
        [self.x, self.y, self.w, self.h]
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct ContainerWnd {
    pub base: Wnd,
    pub children: Vec<Wnd>,
    pub active_child: Option<usize>,
}

#[allow(dead_code)]
impl ContainerWnd {
    pub fn new(x: f32, y: f32, w: f32, h: f32, title: &'static str) -> Self {
        Self {
            base: Wnd::new(x, y, w, h, title),
            children: Vec::new(),
            active_child: None,
        }
    }

    pub fn add_child(&mut self, child: Wnd) -> usize {
        let idx = self.children.len();
        self.children.push(child);
        if self.active_child.is_none() {
            self.active_child = Some(0);
        }
        idx
    }

    pub fn remove_child(&mut self, idx: usize) -> Option<Wnd> {
        if idx < self.children.len() {
            let removed = self.children.remove(idx);
            if self.active_child == Some(idx) {
                self.active_child = if self.children.is_empty() { None } else { Some(0) };
            }
            Some(removed)
        } else {
            None
        }
    }

    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    pub fn get_child(&self, idx: usize) -> Option<&Wnd> {
        self.children.get(idx)
    }

    pub fn get_child_mut(&mut self, idx: usize) -> Option<&mut Wnd> {
        self.children.get_mut(idx)
    }

    pub fn set_active_child(&mut self, idx: usize) {
        if idx < self.children.len() {
            self.active_child = Some(idx);
            for (i, child) in self.children.iter_mut().enumerate() {
                child.set_focused(i == idx);
            }
        }
    }

    pub fn active_child_index(&self) -> Option<usize> {
        self.active_child
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct ListBox {
    pub wnd: Wnd,
    pub items: Vec<String>,
    pub item_height: f32,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub max_visible_rows: usize,
}

#[allow(dead_code)]
impl ListBox {
    pub fn new(x: f32, y: f32, w: f32, h: f32, item_height: f32) -> Self {
        let max_visible_rows = ((h / item_height).floor() as usize).max(1);
        Self {
            wnd: Wnd::new(x, y, w, h, "ListBox"),
            items: Vec::new(),
            item_height,
            selected_index: 0,
            scroll_offset: 0,
            max_visible_rows,
        }
    }

    pub fn add_item(&mut self, item: impl Into<String>) {
        self.items.push(item.into());
    }

    pub fn clear_items(&mut self) {
        self.items.clear();
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    pub fn select_next(&mut self) {
        if !self.items.is_empty() && self.selected_index + 1 < self.items.len() {
            self.selected_index += 1;
            if self.selected_index >= self.scroll_offset + self.max_visible_rows {
                self.scroll_offset += 1;
            }
        }
    }

    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            if self.selected_index < self.scroll_offset {
                self.scroll_offset = self.selected_index;
            }
        }
    }

    pub fn get_selected(&self) -> Option<&str> {
        self.items.get(self.selected_index).map(|s| s.as_str())
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn scrollbar_ratio(&self) -> (f32, f32) {
        if self.items.len() <= self.max_visible_rows {
            (0.0, 1.0)
        } else {
            let thumb_h = self.max_visible_rows as f32 / self.items.len() as f32;
            let thumb_pos = self.scroll_offset as f32 / (self.items.len() - self.max_visible_rows) as f32;
            (thumb_pos, thumb_h)
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct IPhonePad {
    pub battery_level: f32,
    pub signal_bars: u8,
    pub touch_pos: Option<[f32; 2]>,
    pub is_touching: bool,
    pub dpad_direction: Option<[i32; 2]>,
    pub keypad_dial_buffer: String,
    pub cheat_unlocked: Option<&'static str>,
}

impl Default for IPhonePad {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl IPhonePad {
    pub fn new() -> Self {
        Self {
            battery_level: 0.95,
            signal_bars: 5,
            touch_pos: None,
            is_touching: false,
            dpad_direction: None,
            keypad_dial_buffer: String::with_capacity(16),
            cheat_unlocked: None,
        }
    }

    pub fn touch_down(&mut self, pos: [f32; 2]) {
        self.touch_pos = Some(pos);
        self.is_touching = true;
    }

    pub fn touch_move(&mut self, pos: [f32; 2]) {
        if self.is_touching {
            self.touch_pos = Some(pos);
        }
    }

    pub fn touch_up(&mut self) -> Option<[f32; 2]> {
        self.is_touching = false;
        self.touch_pos.take()
    }

    pub fn press_dpad(&mut self, dx: i32, dy: i32) {
        self.dpad_direction = Some([dx.clamp(-1, 1), dy.clamp(-1, 1)]);
    }

    pub fn release_dpad(&mut self) {
        self.dpad_direction = None;
    }

    pub fn dpad(&self) -> Option<[i32; 2]> {
        self.dpad_direction
    }

    pub fn dial_key(&mut self, digit: char) {
        if self.keypad_dial_buffer.len() < 12 {
            self.keypad_dial_buffer.push(digit);
            self.evaluate_cheat_code();
        }
    }

    pub fn clear_dial_buffer(&mut self) {
        self.keypad_dial_buffer.clear();
        self.cheat_unlocked = None;
    }

    pub fn dial_buffer(&self) -> &str {
        &self.keypad_dial_buffer
    }

    pub fn evaluate_cheat_code(&mut self) {
        match self.keypad_dial_buffer.as_str() {
            "555-0100" => self.cheat_unlocked = Some("ARME PACK 1 : PISTOLETS & FUSIL"),
            "555-0150" => self.cheat_unlocked = Some("SANTE & ARMURE MAX (127 HP)"),
            "555-0200" => self.cheat_unlocked = Some("INDICE DE RECHERCHE ZERO"),
            "555-0300" => self.cheat_unlocked = Some("CASH +$10,000"),
            _ => {}
        }
    }

    pub fn drain_battery(&mut self, dt: f32) {
        self.battery_level = (self.battery_level - dt * 0.002).max(0.05);
    }

    pub fn charge_battery(&mut self, dt: f32) {
        self.battery_level = (self.battery_level + dt * 0.05).min(1.0);
    }

    pub fn battery_percentage(&self) -> u32 {
        (self.battery_level * 100.0).round() as u32
    }

    pub fn signal_strength(&self) -> u8 {
        self.signal_bars
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub struct HelpHint {
    pub id: u32,
    pub text: &'static str,
    pub button_prompt: &'static str,
    pub priority: u8,
    pub duration: f32,
    pub timer: f32,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub struct ContextHelp {
    pub queue: Vec<HelpHint>,
    pub current: Option<HelpHint>,
    pub seen_history: Vec<u32>,
}

#[allow(dead_code)]
impl ContextHelp {
    pub fn new() -> Self {
        Self {
            queue: Vec::new(),
            current: None,
            seen_history: Vec::new(),
        }
    }

    pub fn push_hint(&mut self, id: u32, text: &'static str, prompt: &'static str, priority: u8) {
        if self.seen_history.contains(&id) {
            return;
        }
        let hint = HelpHint {
            id,
            text,
            button_prompt: prompt,
            priority,
            duration: 4.5,
            timer: 0.0,
        };
        self.queue.push(hint);
        self.queue.sort_by(|a, b| b.priority.cmp(&a.priority));
        if self.current.is_none() {
            self.advance_queue();
        }
    }

    pub fn show_instant_hint(&mut self, id: u32, text: &'static str, prompt: &'static str) {
        self.current = Some(HelpHint {
            id,
            text,
            button_prompt: prompt,
            priority: 255,
            duration: 3.5,
            timer: 0.0,
        });
        self.seen_history.push(id);
    }

    pub fn update(&mut self, dt: f32) {
        if let Some(hint) = &mut self.current {
            hint.timer += dt;
            if hint.timer >= hint.duration {
                self.advance_queue();
            }
        }
    }

    pub fn advance_queue(&mut self) {
        if let Some(cur) = self.current.take() {
            if !self.seen_history.contains(&cur.id) {
                self.seen_history.push(cur.id);
            }
        }
        if !self.queue.is_empty() {
            self.current = Some(self.queue.remove(0));
        }
    }

    pub fn dismiss_current(&mut self) {
        self.advance_queue();
    }

    pub fn is_active(&self) -> bool {
        self.current.is_some()
    }

    pub fn current_text(&self) -> Option<&'static str> {
        self.current.as_ref().map(|h| h.text)
    }

    pub fn current_prompt(&self) -> Option<&'static str> {
        self.current.as_ref().map(|h| h.button_prompt)
    }

    pub fn current_progress(&self) -> f32 {
        self.current.as_ref().map(|h| (h.timer / h.duration).clamp(0.0, 1.0)).unwrap_or(0.0)
    }

    pub fn clear_all(&mut self) {
        self.queue.clear();
        self.current = None;
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

    #[test]
    fn pda_contacts_list_and_gps_routing() {
        let mut pda = PdaEngine::new();
        assert_eq!(pda.contacts.len(), 8);
        assert_eq!(pda.contacts[0].name, "Uncle Kenny");

        // Open Contacts view and navigate
        pda.current_view = PdaView::ContactsList;
        pda.navigate(1);
        assert_eq!(pda.selected_index, 1);

        // Select contact Chan Jaoming for GPS routing
        let route = pda.select_contact_and_get_gps(1);
        assert!(route.is_some());
        let (pos, name) = route.unwrap();
        assert_eq!(name, "Chan Jaoming");
        assert_eq!(pos, [230.0, 260.0, 5.0]);
        assert_eq!(pda.current_view, PdaView::ContactDetail(1));
    }

    #[test]
    fn wnd_and_container_hierarchy() {
        let mut container = ContainerWnd::new(10.0, 10.0, 300.0, 200.0, "Root");
        assert_eq!(container.child_count(), 0);

        let child1 = Wnd::new(15.0, 25.0, 100.0, 50.0, "Child 1");
        let child2 = Wnd::new(15.0, 85.0, 100.0, 50.0, "Child 2");
        container.add_child(child1);
        container.add_child(child2);

        assert_eq!(container.child_count(), 2);
        assert_eq!(container.active_child_index(), Some(0));

        container.set_active_child(1);
        assert_eq!(container.active_child_index(), Some(1));
        assert!(container.get_child(1).unwrap().is_focused());
        assert!(!container.get_child(0).unwrap().is_focused());

        assert!(container.base.contains(50.0, 50.0));
        assert!(!container.base.contains(500.0, 500.0));
    }

    #[test]
    fn listbox_scrolling_and_selection() {
        let mut list = ListBox::new(0.0, 0.0, 150.0, 100.0, 20.0);
        assert_eq!(list.item_count(), 0);

        for i in 1..=8 {
            list.add_item(format!("Item {}", i));
        }
        assert_eq!(list.item_count(), 8);
        assert_eq!(list.selected_index(), 0);
        assert_eq!(list.get_selected(), Some("Item 1"));

        // Select next down the list
        list.select_next();
        assert_eq!(list.selected_index(), 1);
        assert_eq!(list.get_selected(), Some("Item 2"));

        // Scroll past visible bounds
        list.select_next();
        list.select_next();
        list.select_next();
        list.select_next();
        assert_eq!(list.selected_index(), 5);

        let (thumb_pos, thumb_h) = list.scrollbar_ratio();
        assert!(thumb_h > 0.0 && thumb_h < 1.0);
        assert!(thumb_pos > 0.0);

        // Select previous back up
        list.select_prev();
        assert_eq!(list.selected_index(), 4);
    }

    #[test]
    fn iphone_pad_touch_dpad_and_dialing() {
        let mut pad = IPhonePad::new();
        assert_eq!(pad.battery_percentage(), 95);
        assert_eq!(pad.signal_strength(), 5);

        // Touch simulation
        pad.touch_down([100.0, 150.0]);
        assert!(pad.is_touching);
        pad.touch_move([120.0, 160.0]);
        let released = pad.touch_up();
        assert_eq!(released, Some([120.0, 160.0]));
        assert!(!pad.is_touching);

        // D-pad simulation
        pad.press_dpad(1, 0); // Right
        assert_eq!(pad.dpad(), Some([1, 0]));
        pad.release_dpad();
        assert_eq!(pad.dpad(), None);

        // Keypad dialing and cheat code detection
        for c in "555-0150".chars() {
            pad.dial_key(c);
        }
        assert_eq!(pad.dial_buffer(), "555-0150");
        assert_eq!(pad.cheat_unlocked, Some("SANTE & ARMURE MAX (127 HP)"));

        pad.clear_dial_buffer();
        assert_eq!(pad.dial_buffer(), "");
        assert_eq!(pad.cheat_unlocked, None);

        // Battery drain & charge
        pad.drain_battery(10.0);
        assert!(pad.battery_level < 0.95);
        pad.charge_battery(5.0);
    }

    #[test]
    fn context_help_priority_queue() {
        let mut help = ContextHelp::new();
        assert!(!help.is_active());

        // Push lower priority hint
        help.push_hint(1, "Appuyez pour sauter", "[ESPACE]", 10);
        assert!(help.is_active());
        assert_eq!(help.current_text(), Some("Appuyez pour sauter"));
        assert_eq!(help.current_prompt(), Some("[ESPACE]"));

        // Push higher priority hint (interrupts/preempts)
        help.push_hint(2, "ATTENTION : Véhicule en feu !", "[E] QUITTER", 100);
        help.advance_queue();
        assert_eq!(help.current_text(), Some("ATTENTION : Véhicule en feu !"));

        // Update timer to expire current
        help.update(5.0);
        assert_eq!(help.current_progress(), 0.0);
    }
}
