use std::collections::HashMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RankedSegment {
    pub(crate) bytes: Vec<u8>,
    pub(crate) score: usize,
}

fn stride_for(segment_size: usize) -> usize {
    (segment_size / 2).clamp(1, 64)
}

fn push_segment(
    counts: &mut HashMap<Vec<u8>, usize>,
    sample: &[u8],
    offset: usize,
    segment_size: usize,
) {
    let end = offset.saturating_add(segment_size).min(sample.len());
    let bytes = &sample[offset..end];
    if bytes.len() < 8 {
        return;
    }
    *counts.entry(bytes.to_vec()).or_default() += 1;
}

pub(crate) fn rank_segments(samples: &[&[u8]], segment_size: usize) -> Vec<RankedSegment> {
    let mut counts = HashMap::new();
    let segment_size = segment_size.max(8);
    let stride = stride_for(segment_size);

    for sample in samples {
        if sample.len() < 8 {
            continue;
        }
        if sample.len() <= segment_size {
            push_segment(&mut counts, sample, 0, sample.len());
            continue;
        }

        let mut offset = 0usize;
        while offset + 8 <= sample.len() {
            push_segment(&mut counts, sample, offset, segment_size);
            if offset + segment_size >= sample.len() {
                break;
            }
            offset = offset.saturating_add(stride);
        }

        push_segment(
            &mut counts,
            sample,
            sample.len().saturating_sub(segment_size),
            segment_size,
        );
    }

    let mut ranked = counts
        .into_iter()
        .map(|(bytes, score)| RankedSegment { bytes, score })
        .collect::<Vec<_>>();
    ranked.sort_by(|lhs, rhs| {
        rhs.score
            .cmp(&lhs.score)
            .then_with(|| rhs.bytes.len().cmp(&lhs.bytes.len()))
            .then_with(|| lhs.bytes.cmp(&rhs.bytes))
    });
    ranked
}
