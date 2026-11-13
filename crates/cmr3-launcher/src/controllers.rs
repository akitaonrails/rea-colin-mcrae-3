//! Which controller records the profile holds for a launch.
//!
//! The game keeps one record per controller slot and wipes a record whose
//! USB ids don't match the device in its slot. Devices take slots in
//! `EnumDevices` order: with the virtual wheel shown it comes first (slot
//! 2) and Xidi's pad second (slot 3); without it the pad is in slot 2. So
//! the profile can't hold both records across sessions on its own.
//!
//! Before each launch the launcher puts in each slot the record of the
//! controller that will be there ([`lineup`], [`prepare`]): the record
//! already there when it is that controller's, else the player's own from
//! the side file ([`Stash`], keyed by USB ids), else a seed. The wheel is
//! always seeded when shown (the game would wipe what is in its slot
//! anyway); the pad only in a profile whose controls were never set (as
//! before the wheel). A record taken out of a slot goes to the side file
//! first. The controller that drives is player 1's; the pad behind the
//! wheel belongs to nobody (it would take player 1 over: a later record
//! wins). After the game the side file takes every controller record the
//! profile holds ([`keep`]), the player's in-game tweaks included.

use std::path::Path;
use std::{fs, io};

use rea_launcher::fsx;

use crate::profile::{self, Controller, Device, RECORD_SIZE, Record};

/// The side file, beside the profile in the saves folder.
pub const STASH_FILE: &str = "CMR3.controllers";

/// The side file's first bytes.
const MAGIC: &[u8; 16] = b"rea CMR3 records";

/// A slot and the controller the game will find there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seat {
    pub slot: usize,
    pub controller: Controller,
    /// Player 1 drives with it.
    pub player_one: bool,
}

/// The slots of the launcher's controllers: the virtual wheel first when
/// the game is shown it, then Xidi's pad when it is installed.
pub fn lineup(wheel: bool, pad: bool) -> Vec<Seat> {
    let mut seats = Vec::new();
    for (shown, controller) in [(wheel, Controller::Wheel), (pad, Controller::Pad)] {
        if shown {
            seats.push(Seat {
                slot: profile::FIRST_CONTROLLER + seats.len(),
                controller,
                player_one: seats.is_empty(),
            });
        }
    }
    seats
}

/// The player's controller records kept outside the profile, one per
/// device (by its USB ids).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stash {
    records: Vec<Record>,
}

impl Stash {
    /// The side file's bytes; anything else (missing, damaged, another
    /// format) is an empty stash.
    pub fn parse(bytes: &[u8]) -> Stash {
        let Some(body) = bytes.strip_prefix(MAGIC.as_slice()) else {
            return Stash::default();
        };
        if body.len() % RECORD_SIZE != 0 {
            return Stash::default();
        }
        let records = body
            .chunks(RECORD_SIZE)
            .map(|r| Record(r.to_vec()))
            .filter(|r| r.device().is_some())
            .collect();
        Stash { records }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = MAGIC.to_vec();
        for r in &self.records {
            out.extend_from_slice(&r.0);
        }
        out
    }

    pub fn get(&self, device: Device) -> Option<&Record> {
        self.records.iter().find(|r| r.device() == Some(device))
    }

    /// Keeps `record` as its device's, replacing an older one; a record
    /// with no device is ignored.
    pub fn put(&mut self, record: Record) {
        let Some(device) = record.device() else {
            return;
        };
        match self.records.iter_mut().find(|r| r.device() == Some(device)) {
            Some(old) => *old = record,
            None => self.records.push(record),
        }
    }
}

/// Puts each seat's record in a valid profile (see the module), sealed;
/// what it did, one line per slot changed. The stash takes the records
/// taken out.
pub fn prepare(profile: &mut [u8], seats: &[Seat], stash: &mut Stash) -> Vec<String> {
    if !profile::is_valid(profile) {
        return Vec::new();
    }
    let untouched = profile::is_untouched(profile);
    let mut done = Vec::new();
    for seat in seats {
        let device = seat.controller.device();
        let there = profile::record(profile, seat.slot);
        if there.device() == Some(device) {
            continue;
        }
        let (mut record, how) = match stash.get(device) {
            Some(r) => (r.clone(), "the player's own"),
            None if seat.controller == Controller::Wheel || untouched => {
                (seat.controller.seed(), "seeded")
            }
            None => continue,
        };
        record.set_owner(seat.player_one);
        stash.put(there);
        profile::put_record(profile, seat.slot, &record);
        let player = if seat.player_one {
            "player 1"
        } else {
            "no player"
        };
        done.push(format!(
            "slot {}: {} ({how}, {player})",
            seat.slot,
            seat.controller.name()
        ));
    }
    if !done.is_empty() {
        profile::seal(profile);
    }
    done
}

/// After the game: the stash takes every controller record in a valid
/// profile. True when it changed.
pub fn keep(profile: &[u8], stash: &mut Stash) -> bool {
    if !profile::is_valid(profile) {
        return false;
    }
    let before = stash.clone();
    for slot in profile::controller_slots() {
        stash.put(profile::record(profile, slot));
    }
    *stash != before
}

fn read_stash(path: &Path) -> Stash {
    fs::read(path).map(|b| Stash::parse(&b)).unwrap_or_default()
}

/// [`prepare`] on the profile file at `profile_path` (the runtime copy's)
/// with the side file at `stash_path`; a missing profile is left for the
/// game to create.
pub fn prepare_files(
    profile_path: &Path,
    stash_path: &Path,
    seats: &[Seat],
) -> io::Result<Vec<String>> {
    let Ok(mut bytes) = fs::read(profile_path) else {
        return Ok(Vec::new());
    };
    let mut stash = read_stash(stash_path);
    let before = stash.clone();
    let done = prepare(&mut bytes, seats, &mut stash);
    if stash != before {
        fsx::write_atomic(stash_path, stash.to_bytes())?;
    }
    if !done.is_empty() {
        fsx::install(profile_path, &bytes)?;
    }
    Ok(done)
}

/// [`keep`] from the profile file into the side file; true when the side
/// file changed.
pub fn keep_files(profile_path: &Path, stash_path: &Path) -> io::Result<bool> {
    let Ok(bytes) = fs::read(profile_path) else {
        return Ok(false);
    };
    let mut stash = read_stash(stash_path);
    if !keep(&bytes, &mut stash) {
        return Ok(false);
    }
    fsx::write_atomic(stash_path, stash.to_bytes())?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::tests::new_profile;
    use crate::profile::{is_valid, put_record, record, seal};

    const WHEEL: Controller = Controller::Wheel;
    const PAD: Controller = Controller::Pad;

    fn slot_device(p: &[u8], slot: usize) -> Option<Device> {
        record(p, slot).device()
    }

    /// A record the player tweaked in the game: bindings changed.
    fn tweaked(controller: Controller, mark: u8) -> Record {
        let mut r = controller.seed();
        r.0[0x08] = mark;
        r
    }

    #[test]
    fn the_lineup_follows_the_games_device_order() {
        let seat = |slot, controller, player_one| Seat {
            slot,
            controller,
            player_one,
        };
        assert_eq!(
            lineup(true, true),
            [seat(2, WHEEL, true), seat(3, PAD, false)]
        );
        assert_eq!(lineup(true, false), [seat(2, WHEEL, true)]);
        assert_eq!(lineup(false, true), [seat(2, PAD, true)]);
        assert!(lineup(false, false).is_empty());
    }

    /// No wheel: a new profile gets exactly the pad seed of before.
    #[test]
    fn without_the_wheel_a_new_profile_gets_the_pad_as_before() {
        let mut p = new_profile();
        let mut stash = Stash::default();
        let done = prepare(&mut p, &lineup(false, true), &mut stash);
        assert_eq!(done, ["slot 2: the pad (seeded, player 1)"]);
        assert!(is_valid(&p), "the checksum follows the change");
        assert_eq!(record(&p, 2), PAD.seed());
        let mut expected = new_profile();
        put_record(&mut expected, 2, &PAD.seed());
        seal(&mut expected);
        assert_eq!(p, expected, "nothing else changes");
        assert_eq!(stash, Stash::default(), "an unset record isn't kept");
        assert!(
            prepare(&mut p, &lineup(false, true), &mut stash).is_empty(),
            "only once"
        );
    }

    #[test]
    fn with_the_wheel_a_new_profile_gets_both() {
        let mut p = new_profile();
        let mut stash = Stash::default();
        let done = prepare(&mut p, &lineup(true, true), &mut stash);
        assert_eq!(
            done,
            [
                "slot 2: the wheel (seeded, player 1)",
                "slot 3: the pad (seeded, no player)"
            ]
        );
        assert_eq!(slot_device(&p, 2), Some(WHEEL.device()));
        assert!(record(&p, 2).player_one());
        assert_eq!(slot_device(&p, 3), Some(PAD.device()));
        assert!(
            !record(&p, 3).player_one(),
            "the pad can't take player 1 over"
        );
        assert!(is_valid(&p));
    }

    /// The player's tweaks survive switching the wheel on and off: each
    /// record goes to the side file when it leaves its slot, and comes
    /// back from it.
    #[test]
    fn switching_the_wheel_on_and_off_keeps_the_players_records() {
        let mut p = new_profile();
        put_record(&mut p, 2, &tweaked(PAD, 7));
        seal(&mut p);
        let mut stash = Stash::default();

        // Wheel on: the pad's record moves out, the wheel is seeded, and
        // the pad comes back behind it from the side file.
        let done = prepare(&mut p, &lineup(true, true), &mut stash);
        assert_eq!(done[1], "slot 3: the pad (the player's own, no player)");
        assert_eq!(record(&p, 3).0[0x08], 7);
        // The game ran; the player tweaked the wheel.
        put_record(&mut p, 2, &tweaked(WHEEL, 9));
        seal(&mut p);
        assert!(keep(&p, &mut stash));
        assert!(!keep(&p, &mut stash), "nothing new");

        // Wheel off: the pad is player 1 again, with its tweaks.
        prepare(&mut p, &lineup(false, true), &mut stash);
        let pad = record(&p, 2);
        assert_eq!((pad.device(), pad.0[0x08]), (Some(PAD.device()), 7));
        assert!(pad.player_one());

        // Wheel on again: the wheel's tweaks come back.
        prepare(&mut p, &lineup(true, true), &mut stash);
        assert_eq!(record(&p, 2).0[0x08], 9);
        assert!(record(&p, 2).player_one() && !record(&p, 3).player_one());
        assert!(is_valid(&p));
    }

    /// A profile set up by hand keeps what it has: the record already in
    /// a slot stays as the game saved it, and an older profile set to the
    /// keyboard gets no pad (only the wheel, which the game would
    /// otherwise wipe from its slot).
    #[test]
    fn a_players_setup_stays() {
        let mut p = new_profile();
        let mut wheel = tweaked(WHEEL, 3);
        wheel.set_owner(false); // the player gave player 1 to the keyboard
        put_record(&mut p, 2, &wheel);
        put_record(&mut p, 3, &PAD.seed());
        seal(&mut p);
        let before = p.clone();
        let mut stash = Stash::default();
        assert!(prepare(&mut p, &lineup(true, true), &mut stash).is_empty());
        assert_eq!(p, before);

        let mut keyboard = new_profile();
        keyboard[0x191D5] = 3;
        seal(&mut keyboard);
        let before = keyboard.clone();
        assert!(prepare(&mut keyboard, &lineup(false, true), &mut stash).is_empty());
        assert_eq!(keyboard, before);
        let done = prepare(&mut keyboard, &lineup(true, true), &mut stash);
        assert_eq!(done, ["slot 2: the wheel (seeded, player 1)"]);
    }

    /// A record of a device the launcher doesn't bring (a wheel bound
    /// before, through plain DirectInput) is kept in the side file when
    /// the virtual wheel takes its slot.
    #[test]
    fn another_devices_record_is_kept_when_replaced() {
        let mut p = new_profile();
        let mut fanatec = WHEEL.seed();
        fanatec.0[0x78..0x80].copy_from_slice(&[0x20, 0, 0, 0, 0xb7, 0x0e, 0, 0]);
        put_record(&mut p, 2, &fanatec);
        seal(&mut p);
        let mut stash = Stash::default();
        prepare(&mut p, &lineup(true, false), &mut stash);
        let id = Device {
            vid: 0x0eb7,
            pid: 0x0020,
        };
        assert_eq!(stash.get(id), Some(&fanatec));
    }

    #[test]
    fn a_damaged_profile_is_never_touched() {
        let mut p = new_profile();
        p[0x20] ^= 1;
        let before = p.clone();
        let mut stash = Stash::default();
        assert!(prepare(&mut p, &lineup(true, true), &mut stash).is_empty());
        assert_eq!(p, before);
        assert!(!keep(&p, &mut stash));
        let mut short = vec![0xFF; 100];
        assert!(prepare(&mut short, &lineup(false, true), &mut stash).is_empty());
    }

    #[test]
    fn the_side_file_round_trips_and_refuses_anything_else() {
        let mut stash = Stash::default();
        stash.put(WHEEL.seed());
        stash.put(tweaked(PAD, 1));
        stash.put(tweaked(PAD, 2));
        assert_eq!(stash.get(PAD.device()).unwrap().0[0x08], 2, "the newest");
        assert_eq!(Stash::parse(&stash.to_bytes()), stash);
        let mut torn = stash.to_bytes();
        torn.pop();
        assert_eq!(Stash::parse(&torn), Stash::default());
        assert_eq!(Stash::parse(b"anything"), Stash::default());
    }

    #[test]
    fn the_files_are_written_only_when_something_changes() {
        let dir = std::env::temp_dir().join(format!("cmr3-controllers-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let (profile_path, stash_path) = (dir.join("CMR3"), dir.join(STASH_FILE));
        let seats = lineup(true, true);
        assert!(
            prepare_files(&profile_path, &stash_path, &seats)
                .unwrap()
                .is_empty(),
            "no profile yet"
        );
        assert!(!keep_files(&profile_path, &stash_path).unwrap());
        fs::write(&profile_path, new_profile()).unwrap();
        assert_eq!(
            prepare_files(&profile_path, &stash_path, &seats)
                .unwrap()
                .len(),
            2
        );
        assert!(!stash_path.exists(), "nothing taken out of an unset slot");
        assert!(is_valid(&fs::read(&profile_path).unwrap()));
        assert!(keep_files(&profile_path, &stash_path).unwrap());
        assert!(!keep_files(&profile_path, &stash_path).unwrap());
        let stash = read_stash(&stash_path);
        assert!(stash.get(WHEEL.device()).is_some() && stash.get(PAD.device()).is_some());
        assert!(
            prepare_files(&profile_path, &stash_path, &seats)
                .unwrap()
                .is_empty()
        );
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
        let read = |var| fs::read(std::env::var(var).expect(var)).unwrap();
        let mut ours = read("CMR3_PROFILE_NEW");
        let game = read("CMR3_PROFILE_CHOSEN");
        assert!(is_valid(&ours) && is_valid(&game));
        assert!(!prepare(&mut ours, &lineup(false, true), &mut Stash::default()).is_empty());
        let differ: Vec<usize> = (0..profile::SIZE - 4)
            .filter(|&i| ours[i] != game[i])
            .collect();
        // Only vibration (+0x00), the bindings after the owner (+0x08),
        // and the game's own two-byte counter may differ.
        let first = 0x1DB94;
        let expected = |i: usize| {
            (first..first + 4).contains(&i)
                || (first + 0x08..first + 0x78).contains(&i)
                || (0x429E0..0x429E2).contains(&i)
        };
        assert!(differ.iter().all(|&i| expected(i)), "{differ:x?}");
    }
}
