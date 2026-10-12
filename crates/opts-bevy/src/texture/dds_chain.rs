//! Re-laying of block-compressed DDS mip chains for textures whose size is not a multiple of 4.
//!
//! A DDS file stores level `n` as `ceil(max(1, size >> n) / 4)` blocks per axis of the *file's*
//! size. wgpu requires block-compressed textures to have a multiple of 4 on both axes, so the
//! loader rounds the extent up (`1025 -> 1028`), and the GPU then expects level `n` to have
//! `ceil(max(1, rounded >> n) / 4)` blocks. For many sizes the two layouts differ from some level
//! on (`519 -> 520` differs at level 3), and the loader used to drop the whole chain (Q-TQ03: 2,727
//! of 35,004 DDS chains in the corpus). [`relayout_chain`] copies every level into the layout
//! the GPU expects, repeating the last block row/column where the rounded level is larger.

/// Blocks per axis of a level of a `width` x `height` texture.
fn level_blocks(width: u32, height: u32, level: u32) -> (usize, usize) {
    let (w, h) = ((width >> level).max(1), (height >> level).max(1));
    (w.div_ceil(4) as usize, h.div_ceil(4) as usize)
}

/// Total bytes of `levels` levels of a block-compressed chain.
#[must_use]
pub fn chain_bytes(width: u32, height: u32, levels: u32, block_bytes: usize) -> usize {
    (0..levels)
        .map(|l| {
            let (bw, bh) = level_blocks(width, height, l);
            bw * bh * block_bytes
        })
        .sum()
}

/// Copies a chain stored for `file` (width, height) into the layout of the rounded `gpu` extent.
/// `None` when the file's data is shorter than its own layout.
#[must_use]
pub fn relayout_chain(
    data: &[u8],
    file: (u32, u32),
    gpu: (u32, u32),
    levels: u32,
    block_bytes: usize,
) -> Option<Vec<u8>> {
    if data.len() < chain_bytes(file.0, file.1, levels, block_bytes) {
        return None;
    }
    let mut out = Vec::with_capacity(chain_bytes(gpu.0, gpu.1, levels, block_bytes));
    let mut offset = 0;
    for level in 0..levels {
        let (sbw, sbh) = level_blocks(file.0, file.1, level);
        let (dbw, dbh) = level_blocks(gpu.0, gpu.1, level);
        let src = &data[offset..offset + sbw * sbh * block_bytes];
        for dy in 0..dbh {
            let row = dy.min(sbh - 1) * sbw;
            for dx in 0..dbw {
                let block = (row + dx.min(sbw - 1)) * block_bytes;
                out.extend_from_slice(&src[block..block + block_bytes]);
            }
        }
        offset += src.len();
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_layouts_are_copied_unchanged() {
        let data: Vec<u8> = (0..chain_bytes(64, 64, 4, 8)).map(|i| i as u8).collect();
        assert_eq!(relayout_chain(&data, (64, 64), (64, 64), 4, 8), Some(data));
    }

    #[test]
    fn rounded_extent_changes_the_layout_from_level_3() {
        // 519 -> 520: level 3 has 16 blocks per axis in the file but 17 for the GPU.
        assert_eq!(level_blocks(519, 300, 3), (16, 10));
        assert_eq!(level_blocks(520, 300, 3), (17, 10));
        assert_eq!(level_blocks(519, 300, 2), (33, 19));
        assert_eq!(level_blocks(520, 300, 2), (33, 19));
        // 1025 -> 1028 (the old test case): levels differ although the base blocks agree.
        assert_ne!(
            chain_bytes(256, 1025, 11, 16),
            chain_bytes(256, 1028, 11, 16)
        );
    }

    #[test]
    fn blocks_are_kept_in_place_and_edges_repeated() {
        // File 6x6 (2x2 blocks) rounded to 8x8 (2x2 blocks): same layout.
        // File 9 wide (3 blocks) vs rounded 12 (3 blocks) at level 0, level 1: 4 -> 1 block vs 6 -> 2.
        let (file, gpu, levels, bb) = ((9, 4), (12, 4), 2, 8);
        let mut data = Vec::new();
        for tag in 0..3u8 {
            data.extend([tag; 8]); // level 0: three blocks 0,1,2
        }
        data.extend([9u8; 8]); // level 1: one block
        let out = relayout_chain(&data, file, gpu, levels, bb).expect("relaid");
        assert_eq!(out.len(), chain_bytes(12, 4, 2, 8));
        assert_eq!(&out[0..8], &[0u8; 8]);
        assert_eq!(&out[16..24], &[2u8; 8]);
        // Level 1 of 12x4 is 6x2 -> 2x1 blocks; both repeat the single file block.
        assert_eq!(&out[24..32], &[9u8; 8]);
        assert_eq!(&out[32..40], &[9u8; 8]);
    }

    #[test]
    fn short_data_is_rejected() {
        assert_eq!(relayout_chain(&[0; 10], (64, 64), (64, 64), 3, 8), None);
    }
}
