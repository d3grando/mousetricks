// Mouse Tricks: inventory mouse drags and wheel moves.
//
// Original implementation for Horizon (a horizon-mixins native mod). This file
// is both the standalone, unit-tested Cargo library and the injected
// `crate::game::mousetricks` module. It has no Horizon dependency: the open
// screen is reached only through the small `Menu` trait, which `native.rs`
// implements with the game's own slot clicks (the same clicks a player's
// hand makes, so a server sees nothing but ordinary clicks).

use serde::{Deserialize, Serialize};

/// The mod's id: its row in Options > Mods and its settings folder
/// (`horizon-mods/.horizon-mod-data/mousetricks/`).
pub const ID: &str = "mousetricks";
pub const VERSION: &str = "0.1.0";
pub const DESCRIPTION: &str = "Right-drag places one item per slot, left-drag gathers or moves stacks, the wheel moves items one by one.";
/// The settings, in the mod's settings folder: what the settings page writes.
pub const CONFIG_FILE: &str = "config.json";

/// Which way the wheel sends the hovered stack's items.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WheelDirection {
    /// Down sends one item to the other side, up brings one back.
    Normal,
    /// Up sends, down brings back.
    Inverted,
    /// The item goes the way the wheel turns: up when the other side is
    /// above the hovered slot (the container above the inventory, the main
    /// inventory above the hotbar), down when it is below.
    Positional,
}

/// Where the wheel looks first for an item to bring back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WheelSearch {
    FirstToLast,
    LastToFirst,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct Config {
    /// Right-drag holding a stack: one item into each slot entered, at once,
    /// and one more each time a slot is entered again.
    pub right_drag: bool,
    /// Left-drag with a stack in hand (or picked up by the press): slots of
    /// the same item are gathered into the hand; with Shift, quick-moved.
    pub left_drag: bool,
    /// Shift + left-drag with an empty hand: every slot entered is quick-moved.
    pub shift_drag: bool,
    /// The wheel over a stack moves its items one at a time.
    pub wheel: bool,
    pub wheel_direction: WheelDirection,
    pub wheel_search: WheelSearch,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            right_drag: true,
            left_drag: true,
            shift_drag: true,
            wheel: true,
            wheel_direction: WheelDirection::Normal,
            wheel_search: WheelSearch::LastToFirst,
        }
    }
}
impl Config {
    pub fn from_json(source: &str) -> Result<Self, String> {
        serde_json::from_str(source).map_err(|e| e.to_string())
    }
    /// The file the settings page writes: every key, readable by `from_json`.
    pub fn to_json(&self) -> String {
        let mut json = serde_json::to_string_pretty(self).unwrap_or_default();
        json.push('\n');
        json
    }
}

/// The configuration as read from [`CONFIG_FILE`]: Err when the file is
/// broken (Mouse Tricks then does nothing until it is fixed or reset).
pub type Loaded = Result<Config, String>;

/// One ordinary slot click, as the player's mouse makes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Click {
    /// `PICKUP` with the left button: take the stack, put the hand's down,
    /// merge it, or swap.
    Left,
    /// `PICKUP` with the right button: one item down, or half the stack up.
    Right,
    /// `QUICK_MOVE` (a Shift click): the stack to the other side.
    QuickMove,
}

/// The open screen's slots as Mouse Tricks sees and clicks them.
pub trait Menu {
    type Stack: Clone;
    /// What `slot` holds.
    fn slot(&mut self, slot: usize) -> Option<Self::Stack>;
    /// The stack on the cursor.
    fn carried(&self) -> Option<Self::Stack>;
    fn count(&self, stack: &Self::Stack) -> u32;
    /// The most of this item one slot holds.
    fn max(&self, stack: &Self::Stack) -> u32;
    /// Same item and components (counts aside): they stack.
    fn same(&self, a: &Self::Stack, b: &Self::Stack) -> bool;
    /// One click, through the game's container click path. False when the
    /// screen took none.
    fn click(&mut self, slot: usize, click: Click) -> bool;
    /// The drag becomes the mod's: the game's own drag (and its release's
    /// click) is dropped.
    fn take_over(&mut self);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Left,
    Right,
}
impl Button {
    /// The quick craft type of the game's own drag with this button.
    pub fn drag_type(self) -> i32 {
        match self {
            Self::Left => 0,
            Self::Right => 1,
        }
    }
}

/// A mouse press over an inventory screen, before the game handles it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Press {
    pub button: Button,
    /// The slot under the cursor.
    pub slot: Option<usize>,
    pub shift: bool,
    /// A stack was on the cursor.
    pub holding: bool,
}

/// The cursor moved with the button still down.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Moment {
    /// The slot under the cursor.
    pub hovered: Option<usize>,
    pub shift: bool,
    /// The game's own drag of this button (`quickCraftSlots`), if it runs.
    pub vanilla: Option<Vec<usize>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    /// Right-drag holding a stack.
    Place,
    /// Left-drag over slots of the hand's item.
    Matching,
    /// Shift + left-drag with an empty hand.
    MoveAll,
}

/// A drag Mouse Tricks follows, from press to release.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Drag {
    button: Button,
    mode: Mode,
    press: Option<usize>,
    /// The drag is the mod's (the game's own was dropped).
    owned: bool,
    /// The game's own drag spread first: this one is left to it.
    given_up: bool,
    /// The slot under the cursor at the last move.
    last: Option<usize>,
    /// Slots already gathered or moved (a placing drag places again instead).
    visited: Vec<usize>,
}

impl Drag {
    /// The drag a press may start, with the features `config` turns on.
    pub fn start(press: &Press, config: &Config) -> Option<Self> {
        let mode = match press.button {
            Button::Right if config.right_drag && press.holding && !press.shift => Mode::Place,
            Button::Right => return None,
            Button::Left if press.shift && !press.holding => {
                if !config.shift_drag {
                    return None;
                }
                Mode::MoveAll
            }
            Button::Left if config.left_drag => Mode::Matching,
            Button::Left => return None,
        };
        Some(Self {
            button: press.button,
            mode,
            press: press.slot,
            owned: false,
            given_up: false,
            last: press.slot,
            visited: Vec::new(),
        })
    }

    pub fn button(&self) -> Button {
        self.button
    }

    /// The cursor moved: true when the move is the mod's (the game's own
    /// drag must not grow).
    pub fn moved<M: Menu>(&mut self, menu: &mut M, now: &Moment) -> bool {
        if self.given_up {
            return false;
        }
        let entered = now.hovered.filter(|_| now.hovered != self.last);
        self.last = now.hovered;
        if !self.owned {
            return self.claim(menu, now, entered);
        }
        if let Some(slot) = entered {
            self.enter(menu, slot, now.shift);
        }
        true
    }

    /// Takes the drag over when it is one of the mod's.
    fn claim<M: Menu>(&mut self, menu: &mut M, now: &Moment, entered: Option<usize>) -> bool {
        match self.mode {
            Mode::Place => {
                // Only once the cursor leaves the pressed slot: a right click
                // that stays in it is the game's (it may be a swap).
                let Some(vanilla) = &now.vanilla else {
                    // The game started no drag (or something else took it).
                    self.given_up = true;
                    return false;
                };
                if now.hovered == self.press {
                    return false;
                }
                let first = vanilla.clone();
                self.own(menu);
                for slot in first {
                    place(menu, slot);
                }
                if let Some(slot) = entered {
                    place(menu, slot);
                }
                true
            }
            Mode::Matching => {
                if now.vanilla.as_ref().is_some_and(|v| v.len() > 1) {
                    // The game's drag spread over a second slot first.
                    self.given_up = true;
                    return false;
                }
                let Some(slot) = entered.filter(|&s| Some(s) != self.press) else {
                    return false;
                };
                let Some(held) = menu.carried() else {
                    return false;
                };
                if !menu.slot(slot).is_some_and(|s| menu.same(&s, &held)) {
                    return false;
                }
                self.own(menu);
                if let Some(press) = self.press {
                    self.enter(menu, press, now.shift);
                }
                self.enter(menu, slot, now.shift);
                true
            }
            Mode::MoveAll => {
                let Some(slot) = entered.filter(|&s| Some(s) != self.press) else {
                    return false;
                };
                self.own(menu);
                // The press quick-moved its own slot already.
                self.visited.extend(self.press);
                self.enter(menu, slot, now.shift);
                true
            }
        }
    }

    fn own<M: Menu>(&mut self, menu: &mut M) {
        menu.take_over();
        self.owned = true;
    }

    /// The cursor entered `slot` during the mod's drag.
    fn enter<M: Menu>(&mut self, menu: &mut M, slot: usize, shift: bool) {
        match self.mode {
            Mode::Place => place(menu, slot),
            Mode::Matching => {
                if self.visited.contains(&slot) {
                    return;
                }
                self.visited.push(slot);
                matching(menu, slot, shift);
            }
            Mode::MoveAll => {
                // Without Shift the drag pauses: the slot stays for later.
                if !shift || self.visited.contains(&slot) {
                    return;
                }
                self.visited.push(slot);
                if menu.slot(slot).is_some() {
                    menu.click(slot, Click::QuickMove);
                }
            }
        }
    }
}

/// One item of the hand into `slot`, when it takes it (empty, or the same
/// item with room): never the swap a right click on another item makes.
fn place<M: Menu>(menu: &mut M, slot: usize) {
    let Some(held) = menu.carried() else {
        return;
    };
    let fits = match menu.slot(slot) {
        None => true,
        Some(s) => menu.same(&s, &held) && menu.count(&s) < menu.max(&s),
    };
    if fits {
        menu.click(slot, Click::Right);
    }
}

/// `slot` holds the hand's item: Shift quick-moves it, else it joins the
/// hand when all of it fits.
fn matching<M: Menu>(menu: &mut M, slot: usize, shift: bool) {
    let Some(held) = menu.carried() else {
        return;
    };
    let Some(stack) = menu.slot(slot).filter(|s| menu.same(s, &held)) else {
        return;
    };
    if shift {
        menu.click(slot, Click::QuickMove);
    } else if menu.count(&held) + menu.count(&stack) <= menu.max(&held)
        && menu.click(slot, Click::Left)
    {
        // The hand's stack goes into the slot, then the slot's comes up whole.
        menu.click(slot, Click::Left);
    }
}

/// The open screen's layout for the wheel: the container's slots (300 and
/// up, in order), none for the inventory screen. Slots 0..9 are the hotbar,
/// 9..36 the main inventory.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Layout {
    pub container: Vec<usize>,
}

const HOTBAR: std::ops::Range<usize> = 0..9;
const MAIN: std::ops::Range<usize> = 9..36;

impl Layout {
    /// The slots across from `slot` in quick-move order (where a Shift click
    /// would put its stack): a container's go to the inventory, hotbar first
    /// from its right end; the inventory's go to the container; without
    /// one, the hotbar and the main inventory trade. Empty for other slots.
    pub fn other_side(&self, slot: usize) -> Vec<usize> {
        let player = || HOTBAR.rev().chain(MAIN.rev());
        if self.container.contains(&slot) {
            player().collect()
        } else if !MAIN.contains(&slot) && !HOTBAR.contains(&slot) {
            Vec::new()
        } else if !self.container.is_empty() {
            self.container.clone()
        } else if HOTBAR.contains(&slot) {
            MAIN.collect()
        } else {
            HOTBAR.collect()
        }
    }
    /// The other side's slots in screen order (top left to bottom right).
    fn screen_order(&self, slot: usize) -> Vec<usize> {
        let mut side = self.other_side(slot);
        // The main inventory is drawn above the hotbar.
        side.sort_by_key(|&s| (HOTBAR.contains(&s), s));
        side
    }
    /// `slot` is above the other side on screen.
    pub fn upper(&self, slot: usize) -> bool {
        if self.container.is_empty() {
            MAIN.contains(&slot)
        } else {
            self.container.contains(&slot)
        }
    }
}

/// Wheel steps not yet acted on (a trackpad scrolls in fractions).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Wheel {
    rest: f32,
}
impl Wheel {
    /// Whole steps in `delta` and what is left over; positive is up.
    pub fn steps(&mut self, delta: f32) -> i32 {
        if !delta.is_finite() {
            return 0;
        }
        self.rest += delta;
        let steps = self.rest.trunc();
        self.rest -= steps;
        steps.clamp(-64.0, 64.0) as i32
    }
    pub fn reset(&mut self) {
        self.rest = 0.0;
    }
}

/// `steps` wheel steps over `slot` (positive is up), with an empty hand:
/// each sends one of its items to the other side, or brings one of the same
/// item back, as `config` says. Returns how many items moved.
pub fn scroll<M: Menu>(
    menu: &mut M,
    layout: &Layout,
    config: &Config,
    slot: usize,
    steps: i32,
) -> u32 {
    if steps == 0 || menu.carried().is_some() {
        return 0;
    }
    let up = steps > 0;
    let send = match config.wheel_direction {
        WheelDirection::Normal => !up,
        WheelDirection::Inverted => up,
        WheelDirection::Positional => up != layout.upper(slot),
    };
    let mut moved = 0;
    for _ in 0..steps.unsigned_abs() {
        let done = if send {
            send_one(menu, layout, slot)
        } else {
            bring_one(menu, layout, config.wheel_search, slot)
        };
        if !done {
            break;
        }
        moved += 1;
    }
    moved
}

/// One item of `slot` to the other side: onto a stack of it with room, else
/// into the first empty slot (as a Shift click fills them).
fn send_one<M: Menu>(menu: &mut M, layout: &Layout, slot: usize) -> bool {
    let Some(stack) = menu.slot(slot) else {
        return false;
    };
    let side: Vec<(usize, Option<M::Stack>)> = layout
        .other_side(slot)
        .into_iter()
        .map(|s| (s, menu.slot(s)))
        .collect();
    let target = side
        .iter()
        .find(|(_, s)| {
            s.as_ref()
                .is_some_and(|s| menu.same(s, &stack) && menu.count(s) < menu.max(s))
        })
        .or_else(|| side.iter().find(|(_, s)| s.is_none()))
        .map(|(s, _)| *s);
    let Some(target) = target else {
        return false;
    };
    let before = menu.slot(target).map_or(0, |s| menu.count(&s));
    carry_one(menu, slot, target);
    menu.slot(target).map_or(0, |s| menu.count(&s)) > before
}

/// One item of `slot`'s kind from the other side into `slot`.
fn bring_one<M: Menu>(menu: &mut M, layout: &Layout, search: WheelSearch, slot: usize) -> bool {
    let Some(stack) = menu.slot(slot) else {
        return false;
    };
    if menu.count(&stack) >= menu.max(&stack) {
        return false;
    }
    let mut side = layout.screen_order(slot);
    if search == WheelSearch::LastToFirst {
        side.reverse();
    }
    let source = side
        .into_iter()
        .find(|&s| menu.slot(s).is_some_and(|s| menu.same(&s, &stack)));
    let Some(source) = source else {
        return false;
    };
    let before = menu.count(&stack);
    carry_one(menu, source, slot);
    menu.slot(slot).map_or(0, |s| menu.count(&s)) > before
}

/// Takes `from`'s stack up, puts one item down on `to`, and the rest back.
fn carry_one<M: Menu>(menu: &mut M, from: usize, to: usize) {
    if !menu.click(from, Click::Left) {
        return;
    }
    if menu.carried().is_some() {
        menu.click(to, Click::Right);
    }
    if menu.carried().is_some() {
        menu.click(from, Click::Left);
    }
}

/// A button of the settings page (Options > Mods > Mouse Tricks > Settings).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Setting {
    /// The file is broken: its error. A click reads it again.
    Broken,
    RightDrag,
    LeftDrag,
    ShiftDrag,
    Wheel,
    WheelDirection,
    WheelSearch,
    /// Every setting back to its default.
    Defaults,
}
/// What a click on the settings page asks of the game.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// Write this configuration to the file; it applies at once.
    Save(Config),
    /// Read the file again (after editing it by hand).
    Reload,
}

/// Longest error shown on the page.
const ERROR_WIDTH: usize = 40;

fn on_off(on: bool) -> &'static str {
    if on { "ON" } else { "OFF" }
}

/// The page's buttons, in order. The wheel's options show while it is on.
pub fn page(loaded: &Loaded) -> Vec<Setting> {
    let Ok(c) = loaded else {
        return vec![Setting::Broken, Setting::Defaults];
    };
    let mut page = vec![
        Setting::RightDrag,
        Setting::LeftDrag,
        Setting::ShiftDrag,
        Setting::Wheel,
    ];
    if c.wheel {
        page.extend([Setting::WheelDirection, Setting::WheelSearch]);
    }
    page.push(Setting::Defaults);
    page
}

impl Setting {
    /// Its button's label.
    pub fn label(self, loaded: &Loaded) -> String {
        let c = match loaded {
            Ok(c) => c,
            Err(error) => {
                return match self {
                    Self::Defaults => "Reset to defaults".into(),
                    _ => {
                        let mut short: String = error.chars().take(ERROR_WIDTH).collect();
                        if short.len() < error.len() {
                            short = format!("{}...", short.trim_end());
                        }
                        format!("{CONFIG_FILE}: {short} (click to reload)")
                    }
                };
            }
        };
        match self {
            Self::Broken => String::new(),
            Self::RightDrag => format!("Right-drag: one item per slot: {}", on_off(c.right_drag)),
            Self::LeftDrag => format!("Left-drag: gather same items: {}", on_off(c.left_drag)),
            Self::ShiftDrag => format!("Shift + left-drag: move stacks: {}", on_off(c.shift_drag)),
            Self::Wheel => format!("Wheel: move items one by one: {}", on_off(c.wheel)),
            Self::WheelDirection => match c.wheel_direction {
                WheelDirection::Normal => "Wheel direction: down sends, up brings back",
                WheelDirection::Inverted => "Wheel direction: up sends, down brings back",
                WheelDirection::Positional => "Wheel direction: items follow the wheel",
            }
            .into(),
            Self::WheelSearch => match c.wheel_search {
                WheelSearch::FirstToLast => "Wheel brings back from: first slot",
                WheelSearch::LastToFirst => "Wheel brings back from: last slot",
            }
            .into(),
            Self::Defaults => "Reset to defaults".into(),
        }
    }

    /// What a click on it does to `loaded`.
    pub fn click(self, loaded: &Loaded) -> Effect {
        let Ok(c) = loaded else {
            return match self {
                Self::Defaults => Effect::Save(Config::default()),
                _ => Effect::Reload,
            };
        };
        let mut c = c.clone();
        match self {
            Self::Broken => return Effect::Reload,
            Self::RightDrag => c.right_drag = !c.right_drag,
            Self::LeftDrag => c.left_drag = !c.left_drag,
            Self::ShiftDrag => c.shift_drag = !c.shift_drag,
            Self::Wheel => c.wheel = !c.wheel,
            Self::WheelDirection => {
                c.wheel_direction = match c.wheel_direction {
                    WheelDirection::Normal => WheelDirection::Inverted,
                    WheelDirection::Inverted => WheelDirection::Positional,
                    WheelDirection::Positional => WheelDirection::Normal,
                }
            }
            Self::WheelSearch => {
                c.wheel_search = match c.wheel_search {
                    WheelSearch::FirstToLast => WheelSearch::LastToFirst,
                    WheelSearch::LastToFirst => WheelSearch::FirstToLast,
                }
            }
            Self::Defaults => c = Config::default(),
        }
        Effect::Save(c)
    }
}

/// The settings page's labels (`native_settings`).
pub fn labels(loaded: &Loaded) -> Vec<String> {
    page(loaded).into_iter().map(|s| s.label(loaded)).collect()
}
/// A click on button `button` of the page `labels` gave (`native_settings_click`).
pub fn click(loaded: &Loaded, button: usize) -> Option<Effect> {
    page(loaded).get(button).map(|s| s.click(loaded))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    type Stack = (&'static str, u32);

    /// A chest-and-inventory menu that clicks as the game's menus do.
    #[derive(Default)]
    struct Fake {
        slots: BTreeMap<usize, Stack>,
        carried: Option<Stack>,
        layout: Layout,
        clicks: Vec<(usize, Click)>,
        taken_over: u32,
        /// Slots that refuse items placed into them.
        refusing: Vec<usize>,
    }
    fn max_of(item: &str) -> u32 {
        if item == "pearl" { 16 } else { 64 }
    }
    impl Fake {
        fn chest() -> Self {
            Self {
                layout: Layout {
                    container: (300..327).collect(),
                },
                ..Self::default()
            }
        }
        fn with(mut self, slot: usize, item: &'static str, count: u32) -> Self {
            self.slots.insert(slot, (item, count));
            self
        }
        fn holding(mut self, item: &'static str, count: u32) -> Self {
            self.carried = Some((item, count));
            self
        }
        fn count(&self, slot: usize) -> u32 {
            self.slots.get(&slot).map_or(0, |s| s.1)
        }
        fn total(&self, item: &str) -> u32 {
            let held = self.carried.filter(|c| c.0 == item).map_or(0, |c| c.1);
            held + self
                .slots
                .values()
                .filter(|s| s.0 == item)
                .map(|s| s.1)
                .sum::<u32>()
        }
        fn put(&mut self, slot: usize, stack: Option<Stack>) {
            match stack {
                Some(s) if s.1 > 0 => self.slots.insert(slot, s),
                _ => self.slots.remove(&slot),
            };
        }
        fn quick_move(&mut self, slot: usize) {
            let Some((item, mut count)) = self.slots.get(&slot).copied() else {
                return;
            };
            let side = self.layout.other_side(slot);
            for &t in &side {
                if let Some(s) = self.slots.get_mut(&t)
                    && s.0 == item
                {
                    let room = max_of(item) - s.1;
                    let n = room.min(count);
                    s.1 += n;
                    count -= n;
                }
            }
            if count > 0
                && let Some(&t) = side
                    .iter()
                    .find(|t| !self.slots.contains_key(t) && !self.refusing.contains(t))
            {
                self.slots.insert(t, (item, count));
                count = 0;
            }
            self.put(slot, Some((item, count)));
        }
    }
    impl Menu for Fake {
        type Stack = Stack;
        fn slot(&mut self, slot: usize) -> Option<Stack> {
            self.slots.get(&slot).copied()
        }
        fn carried(&self) -> Option<Stack> {
            self.carried
        }
        fn count(&self, s: &Stack) -> u32 {
            s.1
        }
        fn max(&self, s: &Stack) -> u32 {
            max_of(s.0)
        }
        fn same(&self, a: &Stack, b: &Stack) -> bool {
            a.0 == b.0
        }
        fn click(&mut self, slot: usize, click: Click) -> bool {
            self.clicks.push((slot, click));
            let here = self.slots.get(&slot).copied();
            let refuses = self.refusing.contains(&slot);
            match (click, here, self.carried) {
                (Click::QuickMove, _, _) => self.quick_move(slot),
                (Click::Left, Some(s), None) => {
                    self.carried = Some(s);
                    self.put(slot, None);
                }
                (Click::Right, Some((item, n)), None) => {
                    let up = n.div_ceil(2);
                    self.carried = Some((item, up));
                    self.put(slot, Some((item, n - up)));
                }
                (_, _, Some(_)) if refuses => {}
                (Click::Left, None, Some(c)) => {
                    self.put(slot, Some(c));
                    self.carried = None;
                }
                (Click::Right, None, Some((item, n))) => {
                    self.put(slot, Some((item, 1)));
                    self.carried = (n > 1).then_some((item, n - 1));
                }
                (click, Some((item, n)), Some((held, h))) if item == held => {
                    let moved = match click {
                        Click::Right => 1,
                        _ => h,
                    }
                    .min(max_of(item) - n);
                    self.put(slot, Some((item, n + moved)));
                    self.carried = (h > moved).then_some((item, h - moved));
                }
                (_, Some(s), Some(c)) => {
                    self.put(slot, Some(c));
                    self.carried = Some(s);
                }
                (_, None, None) => return true,
            }
            true
        }
        fn take_over(&mut self) {
            self.taken_over += 1;
        }
    }

    fn press(button: Button, slot: usize, shift: bool, holding: bool) -> Press {
        Press {
            button,
            slot: Some(slot),
            shift,
            holding,
        }
    }
    fn at(slot: Option<usize>, shift: bool, vanilla: Option<Vec<usize>>) -> Moment {
        Moment {
            hovered: slot,
            shift,
            vanilla,
        }
    }

    #[test]
    fn config_defaults_and_file() {
        let c = Config::default();
        assert!(c.right_drag && c.left_drag && c.shift_drag && c.wheel);
        assert_eq!(Config::from_json("{}"), Ok(c.clone()));
        assert_eq!(Config::from_json(&c.to_json()), Ok(c.clone()));
        let json = c.to_json();
        for key in [
            "rightDrag",
            "leftDrag",
            "shiftDrag",
            "wheel",
            "wheelDirection",
            "wheelSearch",
        ] {
            assert!(json.contains(&format!("\"{key}\"")), "{key} in {json}");
        }
        let positional = Config::from_json(r#"{"wheelDirection":"positional","wheel":false}"#);
        assert_eq!(
            positional,
            Ok(Config {
                wheel: false,
                wheel_direction: WheelDirection::Positional,
                ..Config::default()
            })
        );
        assert!(Config::from_json(r#"{"rightdrag":true}"#).is_err());
        assert!(Config::from_json(r#"{"wheelSearch":"middle"}"#).is_err());
        assert!(Config::from_json("not json").is_err());
    }

    #[test]
    fn settings_page_cycles_and_resets() {
        let loaded: Loaded = Ok(Config::default());
        let labels = labels(&loaded);
        assert_eq!(labels.len(), 7);
        assert_eq!(labels[0], "Right-drag: one item per slot: ON");
        assert_eq!(labels[4], "Wheel direction: down sends, up brings back");
        assert_eq!(labels[5], "Wheel brings back from: last slot");
        let Some(Effect::Save(off)) = click(&loaded, 0) else {
            panic!("a toggle saves");
        };
        assert!(!off.right_drag);
        let mut c = Config::default();
        for want in [
            WheelDirection::Inverted,
            WheelDirection::Positional,
            WheelDirection::Normal,
        ] {
            let Some(Effect::Save(next)) = click(&Ok(c.clone()), 4) else {
                panic!("the direction cycles");
            };
            assert_eq!(next.wheel_direction, want);
            c = next;
        }
        // The wheel's options hide while it is off.
        let no_wheel: Loaded = Ok(Config {
            wheel: false,
            ..Config::default()
        });
        assert_eq!(page(&no_wheel).len(), 5);
        assert_eq!(page(&no_wheel)[4], Setting::Defaults);
        assert_eq!(click(&no_wheel, 4), Some(Effect::Save(Config::default())));
        assert_eq!(click(&no_wheel, 9), None);
    }

    #[test]
    fn a_broken_file_shows_its_error() {
        let broken: Loaded = Err("unknown field `rightdrag`, expected one of many".into());
        let labels = labels(&broken);
        assert_eq!(labels.len(), 2);
        assert!(labels[0].starts_with("config.json: unknown field"));
        assert!(labels[0].ends_with("... (click to reload)"));
        assert_eq!(click(&broken, 0), Some(Effect::Reload));
        assert_eq!(click(&broken, 1), Some(Effect::Save(Config::default())));
    }

    #[test]
    fn presses_start_the_drags_their_features_allow() {
        let all = Config::default();
        let none = Config {
            right_drag: false,
            left_drag: false,
            shift_drag: false,
            ..Config::default()
        };
        let cases = [
            (press(Button::Right, 9, false, true), Some(Mode::Place)),
            (press(Button::Right, 9, false, false), None),
            (press(Button::Right, 9, true, true), None),
            (press(Button::Left, 9, false, true), Some(Mode::Matching)),
            (press(Button::Left, 9, false, false), Some(Mode::Matching)),
            (press(Button::Left, 9, true, true), Some(Mode::Matching)),
            (press(Button::Left, 9, true, false), Some(Mode::MoveAll)),
        ];
        for (p, mode) in cases {
            assert_eq!(Drag::start(&p, &all).map(|d| d.mode), mode, "{p:?}");
            assert_eq!(Drag::start(&p, &none), None, "{p:?}");
        }
    }

    #[test]
    fn right_drag_places_one_per_slot_entered() {
        let mut menu = Fake::chest().with(11, "dirt", 5).with(12, "stone", 3);
        menu = menu.holding("dirt", 10);
        let mut drag = Drag::start(&press(Button::Right, 9, false, true), &Config::default())
            .expect("a placing drag");
        // Still in the pressed slot: the game's (a plain right click).
        assert!(!drag.moved(&mut menu, &at(Some(9), false, Some(vec![9]))));
        assert!(menu.clicks.is_empty());
        // Into the next slot: the pressed one and this one get an item each.
        assert!(drag.moved(&mut menu, &at(Some(10), false, Some(vec![9]))));
        assert_eq!(menu.taken_over, 1);
        assert_eq!((menu.count(9), menu.count(10)), (1, 1));
        // A slot of the same item takes one more; another item takes none
        // (no swap).
        assert!(drag.moved(&mut menu, &at(Some(11), false, None)));
        assert!(drag.moved(&mut menu, &at(Some(12), false, None)));
        assert_eq!((menu.count(11), menu.slot(12)), (6, Some(("stone", 3))));
        // Out and back in: one more each time a slot is entered.
        assert!(drag.moved(&mut menu, &at(None, false, None)));
        assert!(drag.moved(&mut menu, &at(Some(10), false, None)));
        assert!(drag.moved(&mut menu, &at(Some(10), false, None)));
        assert_eq!(menu.count(10), 2);
        assert_eq!(menu.carried, Some(("dirt", 6)));
        assert_eq!(menu.total("dirt"), 15);
        assert!(drag.owned);
    }

    #[test]
    fn right_drag_stops_when_the_hand_is_empty() {
        let mut menu = Fake::chest().holding("pearl", 2).with(10, "pearl", 16);
        let mut drag = Drag::start(&press(Button::Right, 9, false, true), &Config::default())
            .expect("a placing drag");
        drag.moved(&mut menu, &at(Some(10), false, Some(vec![9])));
        // Slot 10 is full: only the pressed slot took one.
        assert_eq!((menu.count(9), menu.count(10)), (1, 16));
        drag.moved(&mut menu, &at(Some(11), false, None));
        drag.moved(&mut menu, &at(Some(12), false, None));
        assert_eq!((menu.count(11), menu.count(12)), (1, 0));
        assert_eq!(menu.carried, None);
    }

    #[test]
    fn right_drag_without_the_games_drag_is_left_alone() {
        let mut menu = Fake::chest().holding("dirt", 10);
        let mut drag = Drag::start(&press(Button::Right, 9, false, true), &Config::default())
            .expect("a placing drag");
        assert!(!drag.moved(&mut menu, &at(Some(10), false, None)));
        assert!(!drag.moved(&mut menu, &at(Some(11), false, Some(vec![9]))));
        assert!(menu.clicks.is_empty());
        assert_eq!(menu.taken_over, 0);
    }

    #[test]
    fn left_drag_gathers_the_same_item() {
        let mut menu = Fake::chest()
            .holding("dirt", 5)
            .with(10, "stone", 4)
            .with(11, "dirt", 20)
            .with(12, "dirt", 50)
            .with(13, "dirt", 39);
        let mut drag = Drag::start(&press(Button::Left, 9, false, true), &Config::default())
            .expect("a matching drag");
        // Another item: still the game's.
        assert!(!drag.moved(&mut menu, &at(Some(10), false, Some(vec![9]))));
        assert!(drag.moved(&mut menu, &at(Some(11), false, Some(vec![9]))));
        assert_eq!(menu.carried, Some(("dirt", 25)));
        assert_eq!(menu.count(11), 0);
        // 25 + 50 doesn't fit in a hand; 25 + 39 does.
        assert!(drag.moved(&mut menu, &at(Some(12), false, None)));
        assert!(drag.moved(&mut menu, &at(Some(13), false, None)));
        assert_eq!(menu.carried, Some(("dirt", 64)));
        assert_eq!((menu.count(12), menu.count(13)), (50, 0));
        assert_eq!(menu.total("dirt"), 114);
        assert_eq!(menu.total("stone"), 4);
    }

    #[test]
    fn left_drag_after_picking_a_stack_up() {
        // The press took slot 9's stack up (the game's click); the drag
        // then gathers that item.
        let mut menu = Fake::chest().holding("dirt", 3).with(10, "dirt", 4);
        let mut drag = Drag::start(&press(Button::Left, 9, false, false), &Config::default())
            .expect("a matching drag");
        assert!(drag.moved(&mut menu, &at(Some(10), false, None)));
        assert_eq!(menu.carried, Some(("dirt", 7)));
        // An empty-handed press on an empty slot gathers nothing.
        let mut empty = Fake::chest().with(10, "dirt", 4);
        let mut drag = Drag::start(&press(Button::Left, 9, false, false), &Config::default())
            .expect("a matching drag");
        assert!(!drag.moved(&mut empty, &at(Some(10), false, None)));
        assert!(empty.clicks.is_empty());
    }

    #[test]
    fn left_drag_gives_way_to_the_games_spread() {
        let mut menu = Fake::chest().holding("dirt", 10).with(11, "dirt", 2);
        let mut drag = Drag::start(&press(Button::Left, 9, false, true), &Config::default())
            .expect("a matching drag");
        assert!(!drag.moved(&mut menu, &at(Some(10), false, Some(vec![9]))));
        // The game's drag took slot 10 too: it splits the stack, not the mod.
        assert!(!drag.moved(&mut menu, &at(Some(11), false, Some(vec![9, 10]))));
        assert!(!drag.moved(&mut menu, &at(Some(9), false, None)));
        assert!(menu.clicks.is_empty());
    }

    #[test]
    fn left_drag_with_shift_moves_the_same_item() {
        let mut menu = Fake::chest()
            .holding("dirt", 1)
            .with(9, "dirt", 8)
            .with(10, "dirt", 20)
            .with(11, "stone", 4)
            .with(12, "dirt", 7);
        let mut drag = Drag::start(&press(Button::Left, 9, false, true), &Config::default())
            .expect("a matching drag");
        assert!(drag.moved(&mut menu, &at(Some(10), true, Some(vec![9]))));
        assert!(drag.moved(&mut menu, &at(Some(11), true, None)));
        assert!(drag.moved(&mut menu, &at(Some(12), true, None)));
        // The pressed slot too, since it holds the same item.
        assert_eq!(menu.count(9) + menu.count(10) + menu.count(12), 0);
        assert_eq!(menu.count(300), 35);
        assert_eq!(menu.count(11), 4);
        assert_eq!(menu.carried, Some(("dirt", 1)));
    }

    #[test]
    fn shift_drag_moves_every_stack_entered_once() {
        let mut menu = Fake::chest()
            .with(10, "dirt", 8)
            .with(11, "stone", 4)
            .with(12, "pearl", 3);
        let mut drag = Drag::start(&press(Button::Left, 9, true, false), &Config::default())
            .expect("a moving drag");
        // In the pressed slot (the game's Shift click moved it).
        assert!(!drag.moved(&mut menu, &at(Some(9), true, None)));
        assert!(drag.moved(&mut menu, &at(Some(10), true, None)));
        assert!(drag.moved(&mut menu, &at(Some(11), true, None)));
        // Shift up: the drag pauses; the slot is left for later.
        assert!(drag.moved(&mut menu, &at(Some(12), false, None)));
        assert_eq!(menu.count(12), 3);
        assert!(drag.moved(&mut menu, &at(None, true, None)));
        assert!(drag.moved(&mut menu, &at(Some(12), true, None)));
        assert_eq!(menu.slot(300), Some(("dirt", 8)));
        assert_eq!(menu.slot(301), Some(("stone", 4)));
        assert_eq!(menu.slot(302), Some(("pearl", 3)));
        // Back over a slot that was moved, now refilled: not moved again.
        menu.slots.insert(10, ("dirt", 1));
        drag.moved(&mut menu, &at(None, true, None));
        drag.moved(&mut menu, &at(Some(10), true, None));
        assert_eq!(menu.count(10), 1);
        assert_eq!(menu.taken_over, 1);
    }

    #[test]
    fn layout_follows_quick_move_order() {
        let chest = Layout {
            container: (300..327).collect(),
        };
        let mut player: Vec<usize> = (0..9).rev().collect();
        player.extend((9..36).rev());
        assert_eq!(chest.other_side(305), player);
        assert_eq!(chest.other_side(4), (300..327).collect::<Vec<_>>());
        assert_eq!(chest.other_side(20), (300..327).collect::<Vec<_>>());
        assert!(chest.other_side(100).is_empty());
        assert!(chest.upper(300) && !chest.upper(0) && !chest.upper(20));
        let inventory = Layout::default();
        assert_eq!(inventory.other_side(3), (9..36).collect::<Vec<_>>());
        assert_eq!(inventory.other_side(30), (0..9).collect::<Vec<_>>());
        assert!(inventory.other_side(500).is_empty());
        assert!(inventory.upper(9) && !inventory.upper(8));
        let mut screen: Vec<usize> = (9..36).collect();
        screen.extend(0..9);
        assert_eq!(chest.screen_order(301), screen);
    }

    #[test]
    fn wheel_down_sends_one_item_at_a_time() {
        let mut menu = Fake::chest().with(300, "dirt", 3).with(5, "dirt", 64);
        menu.slots.insert(8, ("dirt", 10));
        let layout = menu.layout.clone();
        let c = Config::default();
        // Onto the hotbar's dirt with room (slot 8, the first in order).
        assert_eq!(scroll(&mut menu, &layout, &c, 300, -2), 2);
        assert_eq!((menu.count(300), menu.count(8)), (1, 12));
        // The last item: the slot empties.
        assert_eq!(scroll(&mut menu, &layout, &c, 300, -5), 1);
        assert_eq!((menu.count(300), menu.count(8)), (0, 13));
        assert_eq!(menu.carried, None);
        assert_eq!(menu.total("dirt"), 77);
    }

    #[test]
    fn wheel_send_fills_the_first_empty_slot_without_a_stack() {
        let mut menu = Fake::chest().with(20, "stone", 5).with(300, "dirt", 1);
        let layout = menu.layout.clone();
        assert_eq!(scroll(&mut menu, &layout, &Config::default(), 20, -1), 1);
        assert_eq!(menu.slot(301), Some(("stone", 1)));
        assert_eq!(menu.count(20), 4);
        // Nowhere to go: nothing moves.
        let mut full = Fake::chest().with(20, "stone", 5);
        for s in 300..327 {
            full.slots.insert(s, ("dirt", 64));
        }
        let layout = full.layout.clone();
        assert_eq!(scroll(&mut full, &layout, &Config::default(), 20, -1), 0);
        assert!(full.clicks.is_empty());
    }

    #[test]
    fn wheel_send_into_a_refusing_slot_puts_everything_back() {
        let mut menu = Fake::chest().with(20, "stone", 5);
        menu.refusing.push(300);
        let layout = menu.layout.clone();
        assert_eq!(scroll(&mut menu, &layout, &Config::default(), 20, -1), 0);
        assert_eq!((menu.count(20), menu.carried), (5, None));
    }

    #[test]
    fn wheel_up_brings_one_back_from_the_search_end() {
        let mut menu = Fake::chest()
            .with(4, "dirt", 10)
            .with(300, "dirt", 2)
            .with(310, "dirt", 2);
        let layout = menu.layout.clone();
        let mut c = Config::default();
        assert_eq!(scroll(&mut menu, &layout, &c, 4, 1), 1);
        assert_eq!(
            (menu.count(300), menu.count(310), menu.count(4)),
            (2, 1, 11)
        );
        c.wheel_search = WheelSearch::FirstToLast;
        assert_eq!(scroll(&mut menu, &layout, &c, 4, 3), 3);
        assert_eq!(
            (menu.count(300), menu.count(310), menu.count(4)),
            (0, 0, 14)
        );
        // Nothing of it left on the other side.
        assert_eq!(scroll(&mut menu, &layout, &c, 4, 1), 0);
        // A full stack brings nothing; an empty slot neither.
        let mut full = Fake::chest().with(4, "pearl", 16).with(300, "pearl", 1);
        let layout = full.layout.clone();
        assert_eq!(scroll(&mut full, &layout, &c, 4, 1), 0);
        assert_eq!(scroll(&mut full, &layout, &c, 5, 1), 0);
        assert!(full.clicks.is_empty());
    }

    #[test]
    fn wheel_directions() {
        let start = || Fake::chest().with(4, "dirt", 10).with(300, "dirt", 10);
        let cases = [
            // (direction, slot, steps, slot 4 after, slot 300 after)
            (WheelDirection::Normal, 4, -1, 9, 11),
            (WheelDirection::Normal, 4, 1, 11, 9),
            (WheelDirection::Inverted, 4, -1, 11, 9),
            (WheelDirection::Inverted, 4, 1, 9, 11),
            // The container is above the inventory: up over the inventory
            // sends up into it, down over the container sends down.
            (WheelDirection::Positional, 4, 1, 9, 11),
            (WheelDirection::Positional, 4, -1, 11, 9),
            (WheelDirection::Positional, 300, -1, 11, 9),
            (WheelDirection::Positional, 300, 1, 9, 11),
        ];
        for (direction, slot, steps, inv, chest) in cases {
            let mut menu = start();
            let layout = menu.layout.clone();
            let c = Config {
                wheel_direction: direction,
                ..Config::default()
            };
            assert_eq!(scroll(&mut menu, &layout, &c, slot, steps), 1);
            assert_eq!(
                (menu.count(4), menu.count(300)),
                (inv, chest),
                "{direction:?} {slot} {steps}"
            );
        }
    }

    #[test]
    fn wheel_trades_hotbar_and_inventory_without_a_container() {
        let mut menu = Fake::default().with(0, "dirt", 2).with(20, "dirt", 1);
        let layout = Layout::default();
        assert_eq!(scroll(&mut menu, &layout, &Config::default(), 0, -1), 1);
        assert_eq!((menu.count(0), menu.count(20)), (1, 2));
        let c = Config {
            wheel_direction: WheelDirection::Positional,
            ..Config::default()
        };
        // The hotbar is below the main inventory: up sends from it.
        assert_eq!(scroll(&mut menu, &layout, &c, 0, 1), 1);
        assert_eq!((menu.count(0), menu.count(20)), (0, 3));
    }

    #[test]
    fn wheel_needs_an_empty_hand() {
        let mut menu = Fake::chest().with(4, "dirt", 10).holding("stone", 1);
        let layout = menu.layout.clone();
        assert_eq!(scroll(&mut menu, &layout, &Config::default(), 4, -1), 0);
        assert!(menu.clicks.is_empty());
    }

    #[test]
    fn wheel_steps_keep_fractions() {
        let mut wheel = Wheel::default();
        assert_eq!(wheel.steps(0.4), 0);
        assert_eq!(wheel.steps(0.4), 0);
        assert_eq!(wheel.steps(0.4), 1);
        assert_eq!(wheel.steps(-3.0), -2);
        assert_eq!(wheel.steps(f32::NAN), 0);
        assert_eq!(wheel.steps(1000.0), 64);
        wheel.reset();
        assert_eq!(wheel.steps(-0.9), 0);
    }
}
