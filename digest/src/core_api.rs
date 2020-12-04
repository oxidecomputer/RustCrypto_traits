use crate::{ExtendableOutput, FixedOutput, Reset, Update, XofReader};
use block_buffer::BlockBuffer;
use generic_array::{ArrayLength, GenericArray};

/// Trait for updating hasher state with input data divided into blocks.
pub trait UpdateCore {
    /// Block size.
    type BlockSize: ArrayLength<u8>;

    /// Update the hasher state using the provided data.
    fn update_blocks(&mut self, blocks: &[GenericArray<u8, Self::BlockSize>]);
}

/// Trait for fixed-output digest implementations to use to retrieve the
/// hash output.
///
/// Usage of this trait in user code is discouraged. Instead use core algorithm
/// wrapped by [`crate::CoreWrapper`], which implements the [`FixedOutput`]
/// trait.
pub trait FixedOutputCore: crate::UpdateCore {
    /// Digest output size.
    type OutputSize: ArrayLength<u8>;

    /// Retrieve result into provided buffer using remaining data stored
    /// in the block buffer and leave hasher in a dirty state.
    ///
    /// This method is expected to only be called once unless [`Reset::reset`]
    /// is called, after which point it can be called again and reset again
    /// (and so on).
    fn finalize_fixed_core(
        &mut self,
        buffer: &mut block_buffer::BlockBuffer<Self::BlockSize>,
        out: &mut GenericArray<u8, Self::OutputSize>,
    );
}

/// Trait for extendable-output function (XOF) core implementations to use to
/// retrieve the hash output.
///
/// Usage of this trait in user code is discouraged. Instead use core algorithm
/// wrapped by [`crate::CoreWrapper`], which implements the
/// [`ExtendableOutput`] trait.
#[cfg(feature = "core-api")]
pub trait ExtendableOutputCore: crate::UpdateCore {
    /// XOF reader.
    type Reader: XofReader;

    /// Retrieve XOF reader using remaining data stored in the block buffer
    /// and leave hasher in a dirty state.
    ///
    /// This method is expected to only be called once unless [`Reset::reset`]
    /// is called, after which point it can be called again and reset again
    /// (and so on).
    fn finalize_xof_core(
        &mut self,
        buffer: &mut block_buffer::BlockBuffer<Self::BlockSize>,
    ) -> Self::Reader;
}

/// Wrapper around core trait implementations.
///
/// It handles data buffering and implements the mid-level traits.
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

impl<D: FixedOutputCore + Reset> FixedOutput for CoreWrapper<D> {
    type OutputSize = D::OutputSize;

    #[inline]
    fn finalize_into(self, out: &mut GenericArray<u8, Self::OutputSize>) {
        let Self {
            mut core,
            mut buffer,
        } = self;
        core.finalize_fixed_core(&mut buffer, out);
    }

    #[inline]
    fn finalize_into_reset(&mut self, out: &mut GenericArray<u8, Self::OutputSize>) {
        let Self { core, buffer } = self;
        core.finalize_fixed_core(buffer, out);
        self.reset();
    }
}

impl<D: ExtendableOutputCore + Reset> ExtendableOutput for CoreWrapper<D> {
    type Reader = D::Reader;

    #[inline]
    fn finalize_xof(self) -> Self::Reader {
        let Self {
            mut core,
            mut buffer,
        } = self;
        core.finalize_xof_core(&mut buffer)
    }

    #[inline]
    fn finalize_xof_reset(&mut self) -> Self::Reader {
        let Self { core, buffer } = self;
        let reader = core.finalize_xof_core(buffer);
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
