use crate::consensus::{self, BlockHeight, NetworkUpgrade};
use orchard::{
    builder::{Builder, Error, InProgress, Unauthorized, Unproven},
    bundle::{Bundle, Flags},
    tree,
};

pub trait UseOrchard {
    fn build<V: core::convert::TryFrom<i64>>(
        self,
        rng: impl rand::RngCore,
    ) -> Option<Result<Bundle<InProgress<Unproven, Unauthorized>, V>, Error>>;

    fn value_balance(&self) -> i64;
}

// Use this if the tx contains orchard
pub struct OrchardBuilder(pub(crate) Option<Builder>);

impl UseOrchard for OrchardBuilder {
    fn build<V: core::convert::TryFrom<i64>>(
        self,
        rng: impl rand::RngCore,
    ) -> Option<Result<Bundle<InProgress<Unproven, Unauthorized>, V>, Error>> {
        self.0.map(|builder| builder.build(rng))
    }

    fn value_balance(&self) -> i64 {
        self.0
            .as_ref()
            .map(|builder| builder.value_balance())
            .unwrap_or(0)
    }
}

impl OrchardBuilder {
    pub fn new<P: consensus::Parameters>(
        params: &P,
        target_height: BlockHeight,
        anchor: tree::Anchor,
    ) -> Self {
        Self(
            match params.is_nu_active(NetworkUpgrade::Nu5, target_height) {
                true => Some(Builder::new(Flags::from_parts(true, true), anchor)),
                false => None,
            },
        )
    }
}

// Use this if there's no orchard in either spend or reciever
pub struct NoOrchardBuilder;
// This is a no-op
impl UseOrchard for NoOrchardBuilder {
    fn build<V: core::convert::TryFrom<i64>>(
        self,
        _: impl rand::RngCore,
    ) -> Option<Result<Bundle<InProgress<Unproven, Unauthorized>, V>, Error>> {
        None
    }

    fn value_balance(&self) -> i64 {
        return 0;
    }
}
