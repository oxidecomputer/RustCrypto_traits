use crate::{
    fixed::{FixedOutput, FixedOutputDirtyCore},
    xof::{ExtendableOutput, ExtendableOutputDirtyCore},
    Reset, Update, UpdateCore,
};
use block_buffer::BlockBuffer;
use generic_array::GenericArray;

/// Wrapper around core algorithms which handles data buffering and
/// implements convinient higher-level traits.
#[derive(Clone, Default)]
pub struct CoreWrapper<D: UpdateCore> {
    core: D,
    buffer: BlockBuffer<D::BlockSize>,
}

impl<D: Reset + UpdateCore> Reset for CoreWrapper<D> {
    #[inline]
    fn reset(&mut self) {
        self.core.reset();
        self.buffer.reset();
    }
}

impl<D: UpdateCore> Update for CoreWrapper<D> {
    #[inline]
    fn update(&mut self, input: &[u8]) {
        let Self { core, buffer } = self;
        buffer.input_blocks(input, |blocks| core.update_blocks(blocks));
    }
}

impl<D: FixedOutputDirtyCore + Reset> FixedOutput for CoreWrapper<D> {
    type OutputSize = D::OutputSize;

    #[inline]
    fn finalize_into(self, out: &mut GenericArray<u8, Self::OutputSize>) {
        let Self { mut core, mut buffer } = self;
        core.finalize_into_dirty_core(&mut buffer, out);
    }

    #[inline]
    fn finalize_into_reset(&mut self, out: &mut GenericArray<u8, Self::OutputSize>) {
        let Self { core, buffer } = self;
        core.finalize_into_dirty_core(buffer, out);
        self.reset();
    }
}

impl<D: ExtendableOutputDirtyCore + Reset> ExtendableOutput for CoreWrapper<D> {
    type Reader = D::Reader;

    #[inline]
    fn finalize_xof(self) -> Self::Reader {
        let Self { mut core, mut buffer } = self;
        core.finalize_xof_dirty_core(&mut buffer)
    }

    #[inline]
    fn finalize_xof_reset(&mut self) -> Self::Reader {
        let Self { core, buffer } = self;
        let reader = core.finalize_xof_dirty_core(buffer);
        self.reset();
        reader
    }
}

#[cfg(feature = "std")]
impl<D: UpdateCore> std::io::Write for CoreWrapper<D> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Update::update(self, buf);
        Ok(buf.len())
    }

    #[inline]
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
