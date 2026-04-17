use std::mem::discriminant;
use std::collections::HashSet;
use crate::libkagami::complex::overrides::ASSOverride;

pub fn transform_inner_tags(ov: &ASSOverride) -> Option<&Vec<ASSOverride>> {
    match ov {
        ASSOverride::TransformI(v)           => Some(v),
        ASSOverride::TransformII(_, v)       => Some(v),
        ASSOverride::TransformIII(_, _, v)   => Some(v),
        ASSOverride::TransformIV(_, _, _, v) => Some(v),
        _ => None,
    }
}

pub fn filter_transform_inner(
    v: Vec<ASSOverride>,
    conflicts: &HashSet<std::mem::Discriminant<ASSOverride>>,
) -> Vec<ASSOverride> {
    v.into_iter()
        .filter(|t| !conflicts.contains(&discriminant(t)))
        .collect()
}

pub fn strip_conflicting_inner_tags(
    ov: ASSOverride,
    conflicts: &HashSet<std::mem::Discriminant<ASSOverride>>,
) -> Option<ASSOverride> {
    match ov {
        ASSOverride::TransformI(v) => {
            let v = filter_transform_inner(v, conflicts);
            if v.is_empty() { None } else { Some(ASSOverride::TransformI(v)) }
        }
        ASSOverride::TransformII(a, v) => {
            let v = filter_transform_inner(v, conflicts);
            if v.is_empty() { None } else { Some(ASSOverride::TransformII(a, v)) }
        }
        ASSOverride::TransformIII(a, b, v) => {
            let v = filter_transform_inner(v, conflicts);
            if v.is_empty() { None } else { Some(ASSOverride::TransformIII(a, b, v)) }
        }
        ASSOverride::TransformIV(a, b, c, v) => {
            let v = filter_transform_inner(v, conflicts);
            if v.is_empty() { None } else { Some(ASSOverride::TransformIV(a, b, c, v)) }
        }
        other => Some(other),
    }
}

/// Invariant: if a raw override tag appears *after* a transform that animates
/// the same variant, that inner tag is stripped from the transform.
/// If the transform ends up with no inner tags, it is dropped entirely.
pub fn apply_same_tag_after_transform(tags: Vec<ASSOverride>) -> Vec<ASSOverride> {
    // Collect per-transform which discriminants get conflicted by later raw tags
    let mut transform_conflicts: Vec<HashSet<std::mem::Discriminant<ASSOverride>>> =
        tags.iter()
            .filter(|t| transform_inner_tags(t).is_some())
            .map(|_| HashSet::new())
            .collect();

    // Map from tag index → transform_conflicts index
    let mut transform_slot: Vec<Option<usize>> = Vec::with_capacity(tags.len());
    let mut slot = 0usize;
    for tag in &tags {
        if transform_inner_tags(tag).is_some() {
            transform_slot.push(Some(slot));
            slot += 1;
        } else {
            transform_slot.push(None);
        }
    }

    // Forward pass: when we see a raw tag, mark all preceding transforms that animate it
    let mut seen_transform_discs: Vec<(usize, Vec<std::mem::Discriminant<ASSOverride>>)> = Vec::new();
    for (i, tag) in tags.iter().enumerate() {
        if let Some(inner) = transform_inner_tags(tag) {
            let discs: Vec<_> = inner.iter().map(|t| discriminant(t)).collect();
            seen_transform_discs.push((i, discs));
        } else {
            let d = discriminant(tag);
            for (ti, discs) in &seen_transform_discs {
                if discs.contains(&d) {
                    if let Some(Some(slot_idx)) = transform_slot.get(*ti) {
                        transform_conflicts[*slot_idx].insert(d);
                    }
                }
            }
        }
    }

    // Apply conflicts — strip inner tags or drop transform entirely
    tags.into_iter()
        .enumerate()
        .filter_map(|(i, tag)| {
            if let Some(Some(slot_idx)) = transform_slot.get(i) {
                let conflicts = &transform_conflicts[*slot_idx];
                if conflicts.is_empty() {
                    Some(tag)
                } else {
                    strip_conflicting_inner_tags(tag, conflicts)
                }
            } else {
                Some(tag)
            }
        })
        .collect()
}