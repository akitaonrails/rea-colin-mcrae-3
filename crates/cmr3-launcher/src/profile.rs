//! The game's profile, `CMR3`: the player's settings and progress, as the
//! game writes them (0x42A28 bytes, the last four a checksum), and the
//! controller records the launcher seeds in it.
//!
//! Layout, read from Rally_3PC.exe 1.1 (the save routine at 0x4142B0, its
//! checksum at 0x412380, the load at 0x4142F0) and checked against
//! profiles the game wrote:
//! - 0x191D4: 18 input records of 0x24E0 bytes, one per DirectInput device
//!   in the game's order: 0 the keyboard (its first word 0 until Keyboard
//!   is chosen), 1 the mouse, 2 on the game controllers in `EnumDevices`
//!   order. On load the game keeps a controller record only when the device
//!   in that slot has the record's USB ids; any other is wiped.
//! - the last four bytes: an MSB-first CRC-32 (polynomial 0x04C11DB7,
//!   all-ones start, inverted) of the whole file with those bytes zeroed.
//!
//! A controller record ([`Record`]): +0x00 force feedback (bit 4 on, bits
//! 5-7 the level; the word is 0 in a new profile), +0x04 the player who
//! owns it (0: player 1, -1: nobody; a later record owned by the same
//! player takes the player over), +0x08 28 bindings (a device control code
//! per game function, -1: the game's default), +0x78 the product id, +0x7C
//! the vendor id, +0x80 35 axis calibrations of 0x20 bytes (zero: the
//! game's), then 256 object calibrations (left zero: the game applies any
//! other to object 0).

use rea_launcher::wheel::dinput::VIRTUAL;

/// The size of every profile the game writes.
pub const SIZE: usize = 0x42A28;
const CHECKSUM: usize = SIZE - 4;
const RECORDS: usize = 0x191D4;
/// The size of one input record.
pub const RECORD_SIZE: usize = 0x24E0;
/// The input records: the keyboard, the mouse, then 16 game controllers.
const SLOTS: usize = 18;
/// The first game controller's record.
pub const FIRST_CONTROLLER: usize = 2;
/// The keyboard's record, whose first word is 0 until Keyboard is chosen.
const KEYBOARD: usize = RECORDS;

// A controller record's fields.
const FFB: usize = 0x00;
const OWNER: usize = 0x04;
const BINDINGS: usize = 0x08;
const PID: usize = 0x78;
const VID: usize = 0x7C;
const CALIBRATIONS: usize = 0x80;
const CALIBRATION_SIZE: usize = 0x20;
/// The owner of a record no player uses.
const NOBODY: i32 = -1;

/// Force feedback at Medium (level 2), as the game stores it.
const FFB_MEDIUM: u32 = 0x50;
/// Force feedback on at the top level (4): the in-game layer applies the
/// player's strength.
const FFB_FULL: u32 = 0x90;

/// The bindings for Xidi's virtual controller with the Rally layout
/// (`xidi`). The game's defaults point steering and the pedals at the
/// right stick; these, saved by the game after remapping on real hardware,
/// put steering (functions 8–9) on the left stick's X axis (12) and the
/// pedals (functions 22–23) on the triggers' Y axis (13), and buttons
/// 0..9 on functions 12..21. Axis codes follow the controller's layout: a
/// d-pad hat would shift them by 2.
const PAD_BINDINGS: [i32; 28] = [
    -1, -1, -1, -1, -1, -1, -1, -1, 12, 12, -1, -1, 0, -1, 1, 3, 4, 5, 6, 7, 8, 9, 13, 13, -1, -1,
    -1, -1,
];

/// The virtual wheel's control codes (buttons 0–31, the hat 32–33, then
/// the axes X 34, Y 35, Z 36, Rz 37): steering (functions 8–9) on X,
/// throttle (22) on Z and brake (23) on Rz, the game's defaults elsewhere.
const STEER: i32 = 34;
const THROTTLE: i32 = 36;
const BRAKE: i32 = 37;

/// One calibration: centre, negative and positive deadzone, minimum,
/// maximum, 0, 100, invert (the game's reader at 0x50A450; values run
/// 0..65536 over the axis).
type Calibration = [i32; 8];
/// Steering read linearly, without the game's ±8000 deadzone (slot 5,
/// which the game reads for a wheel's steering).
const STEERING: (usize, Calibration) = (5, [0x7FFF, 0, 0, -0x7FFF, 0x7FFF, 0, 100, 0]);
/// A pedal resting at the axis minimum, read over the whole axis (slots 7
/// and 8: throttle and brake on separate axes).
const PEDAL: Calibration = [0, 0, 0, 0, 0xFFFF, 0, 100, 0];
const PEDAL_SLOTS: [usize; 2] = [7, 8];

/// A game controller as the game keys its record: by USB ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Device {
    pub vid: u16,
    pub pid: u16,
}

/// The controllers the launcher brings, whose records it seeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Controller {
    /// Xidi's virtual controller 1 (the pad).
    Pad,
    /// The in-game layer's virtual wheel, "rea wheel".
    Wheel,
}

impl Controller {
    pub fn device(self) -> Device {
        match self {
            Controller::Pad => Device {
                vid: 0xFFFF,
                pid: 0x0001,
            },
            Controller::Wheel => Device {
                vid: VIRTUAL.vid,
                pid: VIRTUAL.pid,
            },
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Controller::Pad => "the pad",
            Controller::Wheel => "the wheel",
        }
    }

    /// The record a new profile gets for this controller, owned by
    /// player 1.
    pub fn seed(self) -> Record {
        let mut r = Record(vec![0; RECORD_SIZE]);
        let device = self.device();
        r.put(PID, u32::from(device.pid));
        r.put(VID, u32::from(device.vid));
        r.set_owner(true);
        match self {
            Controller::Pad => {
                r.put(FFB, FFB_MEDIUM);
                r.put_bindings(&PAD_BINDINGS);
            }
            Controller::Wheel => {
                r.put(FFB, FFB_FULL);
                let mut bindings = [-1; 28];
                bindings[8] = STEER;
                bindings[9] = STEER;
                bindings[22] = THROTTLE;
                bindings[23] = BRAKE;
                r.put_bindings(&bindings);
                let (slot, steering) = STEERING;
                r.put_calibration(slot, &steering);
                for slot in PEDAL_SLOTS {
                    r.put_calibration(slot, &PEDAL);
                }
            }
        }
        r
    }
}

/// One input record's bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record(pub Vec<u8>);

impl Record {
    fn word(&self, at: usize) -> u32 {
        word(&self.0, at)
    }

    fn put(&mut self, at: usize, value: u32) {
        put_word(&mut self.0, at, value);
    }

    fn put_bindings(&mut self, bindings: &[i32; 28]) {
        for (i, &code) in bindings.iter().enumerate() {
            self.put(BINDINGS + 4 * i, code as u32);
        }
    }

    fn put_calibration(&mut self, slot: usize, values: &Calibration) {
        for (i, &v) in values.iter().enumerate() {
            self.put(CALIBRATIONS + CALIBRATION_SIZE * slot + 4 * i, v as u32);
        }
    }

    /// The device the record is for; `None` for an empty or wiped one.
    pub fn device(&self) -> Option<Device> {
        let pid = u16::try_from(self.word(PID)).ok()?;
        let vid = u16::try_from(self.word(VID)).ok()?;
        ((pid, vid) != (0, 0)).then_some(Device { vid, pid })
    }

    /// Whether player 1 uses it.
    pub fn player_one(&self) -> bool {
        self.word(OWNER) == 0
    }

    /// Gives it to player 1, or to nobody.
    pub fn set_owner(&mut self, player_one: bool) {
        self.put(OWNER, if player_one { 0 } else { NOBODY as u32 });
    }

    /// Never set: no device, no owner, every binding the game's (a new
    /// profile's record; force feedback may be anything).
    fn is_unset(&self) -> bool {
        (OWNER..CALIBRATIONS)
            .step_by(4)
            .all(|at| self.word(at) == u32::MAX)
    }
}

fn word(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

fn put_word(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

/// CRC-32 as the game computes it: MSB first, polynomial 0x04C11DB7,
/// all-ones start, inverted (CRC-32/BZIP2).
fn crc32(bytes: impl Iterator<Item = u8>) -> u32 {
    !bytes.fold(u32::MAX, |crc, b| {
        (0..8).fold(crc ^ (u32::from(b) << 24), |c, _| {
            if c & 0x8000_0000 != 0 {
                (c << 1) ^ 0x04C1_1DB7
            } else {
                c << 1
            }
        })
    })
}

/// The game's checksum over a whole profile, its stored checksum read as
/// zero.
pub fn checksum(profile: &[u8]) -> u32 {
    let body = profile.iter().take(CHECKSUM).copied();
    crc32(body.chain(std::iter::repeat_n(0, 4)))
}

/// A profile the game would accept: the right size and checksum.
pub fn is_valid(profile: &[u8]) -> bool {
    profile.len() == SIZE && word(profile, CHECKSUM) == checksum(profile)
}

/// Stores the checksum after a change.
pub fn seal(profile: &mut [u8]) {
    let sum = checksum(profile);
    put_word(profile, CHECKSUM, sum);
}

fn span(slot: usize) -> std::ops::Range<usize> {
    let at = RECORDS + slot * RECORD_SIZE;
    at..at + RECORD_SIZE
}

/// The record in `slot` (0..18) of a valid profile.
pub fn record(profile: &[u8], slot: usize) -> Record {
    Record(profile[span(slot)].to_vec())
}

/// Writes `record` into `slot`; [`seal`] afterwards.
pub fn put_record(profile: &mut [u8], slot: usize, record: &Record) {
    profile[span(slot)].copy_from_slice(&record.0);
}

/// The game controller slots.
pub fn controller_slots() -> std::ops::Range<usize> {
    FIRST_CONTROLLER..SLOTS
}

/// A profile whose controls were never set: the keyboard not chosen and
/// the first controller's record unset, with force feedback off.
pub fn is_untouched(profile: &[u8]) -> bool {
    let first = record(profile, FIRST_CONTROLLER);
    word(profile, KEYBOARD) == 0 && first.word(FFB) == 0 && first.is_unset()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A profile as the game starts one: no controls chosen.
    pub(crate) fn new_profile() -> Vec<u8> {
        let mut p = vec![0u8; SIZE];
        let first = span(FIRST_CONTROLLER).start;
        p[first + OWNER..first + CALIBRATIONS].fill(0xFF);
        p[0x10] = 0x42; // some progress elsewhere
        seal(&mut p);
        p
    }

    #[test]
    fn the_checksum_is_the_games() {
        // CRC-32/BZIP2's published check value.
        assert_eq!(crc32(b"123456789".iter().copied()), 0xFC89_1918);
        let mut p = new_profile();
        let sum = checksum(&p);
        p[CHECKSUM] ^= 0xAB;
        assert_eq!(checksum(&p), sum, "the stored checksum reads as zero");
        assert!(!is_valid(&p));
    }

    #[test]
    fn records_sit_where_the_game_keeps_them() {
        assert_eq!(span(0).start, 0x191D4, "the keyboard");
        assert_eq!(span(2).start, 0x1DB94, "the first controller");
        assert_eq!(span(3).start, 0x20074);
        assert_eq!(span(SLOTS - 1).end, 0x191D4 + 18 * 0x24E0);
        assert!(span(SLOTS - 1).end <= CHECKSUM);
        assert_eq!(
            CALIBRATIONS + 35 * CALIBRATION_SIZE + 256 * 0x20,
            RECORD_SIZE
        );
    }

    /// The wheel's seed at the offsets read from Rally_3PC.exe 1.1, as
    /// they land in the profile with the wheel first (slot 2, 0x1DB94).
    #[test]
    fn the_wheel_seed_is_byte_for_byte_the_verified_one() {
        let mut p = new_profile();
        put_record(&mut p, 2, &Controller::Wheel.seed());
        let at = |offset: usize| word(&p, offset);
        assert_eq!(p[0x1DB94], 0x90, "force feedback on, level 4");
        assert_eq!(p[0x1DB95], 0, "analogue steering and pedals, not flipped");
        assert_eq!(at(0x1DB98), 0, "player 1");
        assert_eq!((at(0x1DBBC), at(0x1DBC0)), (34, 34), "steering on X");
        assert_eq!((at(0x1DBF4), at(0x1DBF8)), (36, 37), "throttle Z, brake Rz");
        let set = [0x1DBBC, 0x1DBC0, 0x1DBF4, 0x1DBF8];
        for f in (0x1DB9C..0x1DC0C).step_by(4).filter(|o| !set.contains(o)) {
            assert_eq!(at(f), u32::MAX, "function at {f:#x}: the game's default");
        }
        assert_eq!((at(0x1DC0C), at(0x1DC10)), (0x5245, 0x1209), "PID, VID");
        let slot = |o: usize| (0..8).map(|i| at(o + 4 * i) as i32).collect::<Vec<_>>();
        assert_eq!(slot(0x1DCB4), [0x7FFF, 0, 0, -0x7FFF, 0x7FFF, 0, 100, 0]);
        assert_eq!(slot(0x1DCF4), [0, 0, 0, 0, 0xFFFF, 0, 100, 0]);
        assert_eq!(slot(0x1DD14), [0, 0, 0, 0, 0xFFFF, 0, 100, 0]);
        let calibrated = |o: usize| {
            [0x1DCB4, 0x1DCF4, 0x1DD14]
                .iter()
                .any(|s| (s..&(s + 0x20)).contains(&&o))
        };
        for o in (0x1DC14..0x1DB94 + RECORD_SIZE).filter(|o| !calibrated(*o)) {
            assert_eq!(p[o], 0, "calibration byte {o:#x}");
        }
    }

    #[test]
    fn the_pad_seed_is_the_one_validated_on_xidi() {
        let pad = Controller::Pad.seed();
        assert_eq!(pad.word(FFB), 80, "vibration at Medium");
        assert!(pad.player_one());
        assert_eq!(pad.word(BINDINGS + 4 * 8), 12, "steering on the left stick");
        assert_eq!(pad.word(BINDINGS + 4 * 22), 13, "pedals on the triggers");
        assert_eq!((pad.word(PID), pad.word(VID)), (1, 0xFFFF));
        assert!(pad.0[CALIBRATIONS..].iter().all(|&b| b == 0));
        assert_eq!(pad.device(), Some(Controller::Pad.device()));
    }

    #[test]
    fn devices_and_owners_read_back() {
        let p = new_profile();
        assert_eq!(record(&p, 2).device(), None, "unset");
        assert_eq!(record(&p, 5).device(), None, "zeroed");
        let mut wheel = Controller::Wheel.seed();
        assert_eq!(
            wheel.device(),
            Some(Device {
                vid: 0x1209,
                pid: 0x5245
            })
        );
        wheel.set_owner(false);
        assert!(!wheel.player_one());
        assert_eq!(wheel.word(OWNER), u32::MAX);
    }

    #[test]
    fn a_players_choice_is_not_untouched() {
        assert!(is_untouched(&new_profile()));
        let mut keyboard = new_profile();
        keyboard[KEYBOARD + 1] = 3;
        assert!(!is_untouched(&keyboard));
        let mut seeded = new_profile();
        put_record(&mut seeded, 2, &Controller::Pad.seed());
        assert!(!is_untouched(&seeded));
    }
}
