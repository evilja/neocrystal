use rand::Rng;
use rand::seq::SliceRandom;

use crate::modules::preferences::BASE_SCORE;

/// Build one complete automatic shuffle cycle.
///
/// The returned values are the original song indexes.  The planner deliberately
/// knows nothing about the song objects themselves, which keeps its invariants
/// straightforward to test with a seeded RNG.
pub fn plan<R: Rng + ?Sized>(
    entries: &[(usize, u8)],
    first_loop: bool,
    just_finished: Option<usize>,
    rng: &mut R,
) -> Vec<usize> {
    if entries.is_empty() {
        return Vec::new();
    }

    let mut low = Vec::new();
    let mut normal = Vec::new();
    let mut high = Vec::new();
    for &(index, score) in entries {
        if score > BASE_SCORE {
            high.push((index, score));
        } else if score < BASE_SCORE {
            low.push((index, score));
        } else {
            normal.push((index, score));
        }
    }

    low.shuffle(rng);
    normal.shuffle(rng);

    let mut slots = vec![None; entries.len()];
    let low_positions = reserve_positions(low.len(), entries.len(), false, &mut slots, rng);
    for (position, &(index, _)) in low_positions.iter().zip(low.iter()) {
        slots[*position] = Some(index);
    }

    let high_positions = reserve_positions(high.len(), entries.len(), first_loop, &mut slots, rng);
    let high_order = weighted_high_order(&high, rng);
    for (position, index) in high_positions.iter().zip(high_order) {
        slots[*position] = Some(index);
    }

    let mut normals = normal.into_iter().map(|(index, _)| index);
    for slot in &mut slots {
        if slot.is_none() {
            *slot = normals.next();
        }
    }

    // The reservation steps always leave exactly as many free positions as
    // there are normal tracks.  The fallback is useful if this function is
    // later changed to use tier-specific reservations with collisions.
    let remaining: Vec<usize> = entries
        .iter()
        .map(|&(index, _)| index)
        .filter(|index| !slots.contains(&Some(*index)))
        .collect();
    let mut remaining = remaining.into_iter();
    for slot in &mut slots {
        if slot.is_none() {
            *slot = remaining.next();
        }
    }

    let mut result: Vec<usize> = slots.into_iter().flatten().collect();
    if result.len() > 1
        && let Some(last) = just_finished
        && result[0] == last
    {
        // A one-position swap minimally changes the carefully spaced plan
        // while preventing an automatic boundary repeat.
        result.swap(0, 1);
    }
    result
}

fn reserve_positions<R: Rng + ?Sized>(
    count: usize,
    loop_len: usize,
    first_loop: bool,
    slots: &mut [Option<usize>],
    rng: &mut R,
) -> Vec<usize> {
    if count == 0 || loop_len == 0 {
        return Vec::new();
    }

    let mut positions = Vec::with_capacity(count);
    for ordinal in 0..count {
        let target = if first_loop {
            weighted_quantile(ordinal, count, loop_len)
        } else {
            uniform_quantile(ordinal, count, loop_len)
        };
        let position = nearest_free_slot(target, slots, rng);
        positions.push(position);
    }
    positions.sort_unstable();
    positions
}

fn uniform_quantile(ordinal: usize, count: usize, loop_len: usize) -> f64 {
    ((ordinal as f64 + 0.5) * loop_len as f64 / count as f64 - 0.5)
        .clamp(0.0, (loop_len - 1) as f64)
}

/// A modest 1.15x density in the first 100 positions.  Expressing the bias
/// as a weighted quantile avoids a hard cutoff and keeps every position valid.
fn weighted_quantile(ordinal: usize, count: usize, loop_len: usize) -> f64 {
    let early_len = loop_len.min(100) as f64;
    let early_weight = early_len * 1.15;
    let total_weight = early_weight + loop_len.saturating_sub(100) as f64;
    let weighted = (ordinal as f64 + 0.5) * total_weight / count as f64;
    if weighted < early_weight {
        (weighted / 1.15 - 0.5).clamp(0.0, (loop_len - 1) as f64)
    } else {
        (early_len + weighted - early_weight - 0.5).clamp(0.0, (loop_len - 1) as f64)
    }
}

fn nearest_free_slot<R: Rng + ?Sized>(target: f64, slots: &[Option<usize>], rng: &mut R) -> usize {
    let distance = |position: usize| (position as f64 - target).abs();
    let best = slots
        .iter()
        .enumerate()
        .filter(|(_, slot)| slot.is_none())
        .map(|(position, _)| distance(position))
        .fold(f64::INFINITY, f64::min);

    let candidates: Vec<usize> = slots
        .iter()
        .enumerate()
        .filter(|(_, slot)| slot.is_none())
        .filter(|(position, _)| (distance(*position) - best).abs() < f64::EPSILON)
        .map(|(position, _)| position)
        .collect();
    candidates[rng.random_range(0..candidates.len())]
}

fn weighted_high_order<R: Rng + ?Sized>(entries: &[(usize, u8)], rng: &mut R) -> Vec<usize> {
    let mut remaining = entries.to_vec();
    let mut result = Vec::with_capacity(remaining.len());
    while !remaining.is_empty() {
        let total: f64 = remaining
            .iter()
            .map(|&(_, score)| 1.0 + f64::from(score.saturating_sub(BASE_SCORE)))
            .sum();
        let mut choice = rng.random::<f64>() * total;
        let mut selected = remaining.len() - 1;
        for (position, &(_, score)) in remaining.iter().enumerate() {
            choice -= 1.0 + f64::from(score.saturating_sub(BASE_SCORE));
            if choice <= 0.0 {
                selected = position;
                break;
            }
        }
        result.push(remaining.swap_remove(selected).0);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn entries(count: usize) -> Vec<(usize, u8)> {
        (0..count).map(|index| (index, BASE_SCORE)).collect()
    }

    #[test]
    fn plan_is_a_permutation_for_edge_sizes() {
        for count in [0, 1, 2, 3, 10, 250] {
            let input = entries(count);
            let mut rng = StdRng::seed_from_u64(count as u64);
            let output = plan(&input, true, None, &mut rng);
            let mut sorted = output.clone();
            sorted.sort_unstable();
            assert_eq!(sorted, (0..count).collect::<Vec<_>>());
        }
    }

    #[test]
    fn low_tracks_are_spaced() {
        let input: Vec<_> = (0..24).map(|index| (index, 80)).collect();
        let mut rng = StdRng::seed_from_u64(4);
        let output = plan(&input, false, None, &mut rng);
        let mut positions: Vec<_> = output
            .iter()
            .enumerate()
            .filter(|(_, index)| **index < 24)
            .map(|(position, _)| position)
            .collect();
        positions.sort_unstable();
        assert!(
            positions
                .windows(2)
                .all(|window| window[1] - window[0] <= 2)
        );
    }

    #[test]
    fn high_scores_favor_earlier_high_slots() {
        let input: Vec<_> = (0..20)
            .map(|index| (index, if index == 0 { 255 } else { 101 }))
            .collect();
        let mut rng = StdRng::seed_from_u64(12);
        let output = plan(&input, false, None, &mut rng);
        assert!(output.iter().position(|index| *index == 0).unwrap() < 20);
    }

    #[test]
    fn first_loop_bias_has_more_early_high_tracks() {
        let input: Vec<_> = (0..200)
            .map(|index| (index, if index < 50 { 120 } else { BASE_SCORE }))
            .collect();
        let mut first_rng = StdRng::seed_from_u64(20);
        let mut later_rng = StdRng::seed_from_u64(20);
        let first = plan(&input, true, None, &mut first_rng);
        let later = plan(&input, false, None, &mut later_rng);
        let first_early = first[..100].iter().filter(|index| **index < 50).count();
        let later_early = later[..100].iter().filter(|index| **index < 50).count();
        assert!(first_early >= later_early);
    }

    #[test]
    fn boundary_does_not_repeat() {
        let input = entries(5);
        let mut found = false;
        for seed in 0..100 {
            let mut rng = StdRng::seed_from_u64(seed);
            let output = plan(&input, false, Some(0), &mut rng);
            assert_ne!(output[0], 0);
            found = true;
        }
        assert!(found);
    }
}
