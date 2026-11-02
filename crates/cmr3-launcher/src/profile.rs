//! The game's profile, `CMR3`: the player's settings and progress, as the
//! game writes them (0x42A28 bytes, the last four a checksum).
//!
//! A new profile has no game controller chosen, so a pad does nothing
//! until the player picks Xidi's controller in Options → Controls. With a
//! Xidi preset on, the launcher makes that choice for a profile nobody has
//! configured yet, with vibration at Medium; any profile whose controls
//! were set by hand, keyboard included, stays as the player left it.
//!
//! Layout, read from Rally_3PC.exe 1.1 (the save routine at 0x4142B0 and
//! its checksum at 0x412380) and checked against profiles the game wrote:
//! - 0x191D4: player 1's keyboard block, 0 until Keyboard is chosen.
//! - 0x1DB94: player 1's vibration strength: 0 off, 80 Medium.
//! - 0x1DB98: player 1's joystick block, 31 words: the DirectInput device
//!   index (-1: none), then its bindings. All -1 in a new profile.
//! - the last four bytes: an MSB-first CRC-32 (polynomial 0x04C11DB7,
//!   all-ones start, inverted) of the whole file with those bytes zeroed.

use std::{fs, io, path::Path};

use crate::runtime;

/// The size of every profile the game writes.
pub const SIZE: usize = 0x42A28;
const CHECKSUM: usize = SIZE - 4;
const KEYBOARD: usize = 0x191D4;
const VIBRATION: usize = 0x1DB94;
const JOYSTICK: usize = 0x1DB98;

/// Vibration at Medium, as the game stores it.
const VIBRATION_MEDIUM: u32 = 80;

/// Player 1's joystick block for the first controller (Xidi's virtual
/// controller 1): device 0 and bindings for the Rally layout (`xidi`).
/// The game's own defaults point steering and the pedals at the right
/// stick; these, saved by the game after remapping on real hardware, put
/// steering (words 9–10) on the left stick's X axis (12) and the pedals
/// (words 23–24) on the triggers' Y axis (13). Axis codes follow the
/// controller's layout: a d-pad hat would shift them by 2.
const FIRST_CONTROLLER: [i32; 31] = [
    0, -1, -1, -1, -1, -1, -1, -1, -1, 12, 12, -1, -1, 0, -1, 1, 3, 4, 5, 6, 7, 8, 9, 13, 13, -1,
    -1, -1, -1, 1, 65535,
];

fn word(profile: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(profile[at..at + 4].try_into().unwrap())
}

fn put_word(profile: &mut [u8], at: usize, value: u32) {
    profile[at..at + 4].copy_from_slice(&value.to_le_bytes());
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

/// Chooses the first game controller, with vibration at Medium, in a
/// valid profile whose controls were never set, as the game does when the
/// player picks them; returns whether it did.
pub fn choose_first_controller(profile: &mut [u8]) -> bool {
    let untouched = is_valid(profile)
        && word(profile, KEYBOARD) == 0
        && word(profile, VIBRATION) == 0
        && (0..FIRST_CONTROLLER.len()).all(|i| word(profile, JOYSTICK + 4 * i) == u32::MAX);
    if !untouched {
        return false;
    }
    for (i, &v) in FIRST_CONTROLLER.iter().enumerate() {
        put_word(profile, JOYSTICK + 4 * i, v as u32);
    }
    put_word(profile, VIBRATION, VIBRATION_MEDIUM);
    put_word(profile, CHECKSUM, checksum(profile));
    true
}

/// The same for the profile file at `path` (the runtime copy's), written
/// whole; a missing file is left for the game to create.
pub fn choose_first_controller_in(path: &Path) -> io::Result<bool> {
    let Ok(mut profile) = fs::read(path) else {
        return Ok(false);
    };
    if !choose_first_controller(&mut profile) {
        return Ok(false);
    }
    runtime::install(path, &profile)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A profile as the game starts one: no controls chosen.
    fn new_profile() -> Vec<u8> {
        let mut p = vec![0u8; SIZE];
        p[JOYSTICK..JOYSTICK + 4 * FIRST_CONTROLLER.len()].fill(0xFF);
        p[0x10] = 0x42; // some progress elsewhere
        let sum = checksum(&p);
        put_word(&mut p, CHECKSUM, sum);
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
    fn a_new_profile_gets_the_first_controller() {
        let mut p = new_profile();
        assert!(choose_first_controller(&mut p));
        assert!(is_valid(&p), "the checksum follows the change");
        assert_eq!(word(&p, JOYSTICK), 0, "device 0: Xidi's controller 1");
        assert_eq!(word(&p, JOYSTICK + 4 * 9), 12, "steering on the left stick");
        assert_eq!(word(&p, JOYSTICK + 4 * 23), 13, "pedals on the triggers");
        assert_eq!(word(&p, VIBRATION), VIBRATION_MEDIUM);
        assert_eq!(p[0x10], 0x42, "nothing else changes");
        assert!(!choose_first_controller(&mut p), "only once");
    }

    #[test]
    fn a_players_choice_stays() {
        // Keyboard chosen in the game.
        let mut keyboard = new_profile();
        keyboard[KEYBOARD + 1] = 3;
        let sum = checksum(&keyboard);
        put_word(&mut keyboard, CHECKSUM, sum);
        let before = keyboard.clone();
        assert!(!choose_first_controller(&mut keyboard));
        assert_eq!(keyboard, before);
        // A damaged or foreign file is never touched.
        let mut damaged = new_profile();
        damaged[0x20] ^= 1;
        assert!(!choose_first_controller(&mut damaged));
        let mut short = vec![0xFF; 100];
        assert!(!choose_first_controller(&mut short));
    }

    #[test]
    fn the_file_is_rewritten_once() {
        let dir = std::env::temp_dir().join(format!("cmr3-profile-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("CMR3");
        assert!(
            !choose_first_controller_in(&path).unwrap(),
            "no profile yet"
        );
        fs::write(&path, new_profile()).unwrap();
        assert!(choose_first_controller_in(&path).unwrap());
        assert!(!choose_first_controller_in(&path).unwrap());
        assert!(is_valid(&fs::read(&path).unwrap()));
        fs::remove_dir_all(&dir).unwrap();
    }

    /// Against profiles the game wrote (never committed): a new one, and
    /// the same after choosing the first controller in Options → Controls
    /// (vibration left off there; ours turns it to Medium, and binds the
    /// validated axes instead of the game's defaults).
    ///   CMR3_PROFILE_NEW=… CMR3_PROFILE_CHOSEN=… cargo test -- --ignored
    #[test]
    #[ignore]
    fn matches_the_games_own_choice() {
        let read = |var| std::fs::read(std::env::var(var).expect(var)).unwrap();
        let mut ours = read("CMR3_PROFILE_NEW");
        let game = read("CMR3_PROFILE_CHOSEN");
        assert!(is_valid(&ours) && is_valid(&game));
        assert!(choose_first_controller(&mut ours));
        let differ: Vec<usize> = (0..CHECKSUM).filter(|&i| ours[i] != game[i]).collect();
        // Only vibration, the bindings after the device index, and the
        // game's own two-byte counter may differ.
        let bindings = JOYSTICK + 4..JOYSTICK + 4 * FIRST_CONTROLLER.len();
        let expected = |i: usize| {
            (VIBRATION..VIBRATION + 4).contains(&i)
                || bindings.contains(&i)
                || (0x429E0..0x429E2).contains(&i)
        };
        assert!(differ.iter().all(|&i| expected(i)), "{differ:x?}");
        assert_eq!(word(&ours, VIBRATION), VIBRATION_MEDIUM);
    }
}
