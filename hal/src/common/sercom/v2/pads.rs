//! Version 2 of the SERCOM pads module
//!
//! This module implements the [`Pad`] type, which represents a [`Pin`]
//! configured to act as a SERCOM pad. A [`Pad`] is parameterized by three
//! types. The first two types identify the pad by its [`Sercom`] and
//! [`PadNum`]. However, each SERCOM pad can usually be mapped to several
//! possible GPIO pins. The third type must implement the [`Map`] trait, which
//! identifies a corresponding [`PinId`] and [`PinMode`]. The [`PinMode`] is
//! usually [`AlternateC`] or [`AlternateD`].
//!
//! To create a [`Pad`], use the [`From`]/[`Into`] traits. Upon creation, the
//! [`Pad`] takes ownership of the [`Pin`]. The conversion from [`Pin`] to
//! [`Pad`] is potentially many-valued, so it usually must be constrained. On
//! the other hand, the conversion from [`Pad`] to [`Pin`] is always unique,
//! because the [`Pad`] always knows which [`Pin`] it contains.
//!
//! ```
//! let pad: Pad<Sercom0, Pad0, IoSet1> = pins.pa08.into();
//! let pin: Pin<_, _> = pad.into();
//! ```
//!
//! Because of differences in the way pins are mapped to SERCOM pads, the
//! [`Map`] trait is implemented on different types, depending on the chip
//! series. See the [`Map`] documentation for more details.
//!
//! As a result, the actual implementations of [`Map`] are not found in this
//! module. They are included in the [`pad_map`] module.
//!
//! [`pad_map`]: crate::sercom::v2::pad_map

use core::mem::transmute;
use core::ops::Deref;

use crate::paste::paste;

use crate::target_device::sercom0;
use crate::target_device::{SERCOM0, SERCOM1};
#[cfg(any(feature = "samd21", feature = "min-samd51g"))]
use crate::target_device::{SERCOM2, SERCOM3};
#[cfg(any(feature = "min-samd21g", feature = "min-samd51g"))]
use crate::target_device::{SERCOM4, SERCOM5};
#[cfg(feature = "min-samd51n")]
use crate::target_device::{SERCOM6, SERCOM7};

use crate::gpio::v2::*;
use crate::typelevel::*;

//==============================================================================
//  Sercom
//==============================================================================

/// Type-level `enum` representing a Serial Communication Interface (SERCOM)
pub trait Sercom: Sealed {
    /// Corresponding [PAC](crate::target_device) SERCOM type
    type SERCOM: Deref<Target = sercom0::RegisterBlock>;
}

/// Type alias to extract the correct [PAC](crate::target_device) SERCOM type
/// from the [`Sercom`] instance
pub type SERCOM<S> = <S as Sercom>::SERCOM;

macro_rules! sercom {
    ( $($Sercom:ident),+ ) => {
        paste! {
            $(
                /// Represents the corresponding SERCOM instance
                pub enum $Sercom {}
                impl Sealed for $Sercom {}
                impl Sercom for $Sercom { type SERCOM = [<$Sercom:upper>]; }
            )+
        }
    };
}

sercom!(Sercom0, Sercom1);
#[cfg(any(feature = "samd21", feature = "min-samd51g"))]
sercom!(Sercom2, Sercom3);
#[cfg(any(feature = "min-samd21g", feature = "min-samd51g"))]
sercom!(Sercom4, Sercom5);
#[cfg(feature = "min-samd51n")]
sercom!(Sercom6, Sercom7);

//==============================================================================
//  PadNum
//==============================================================================

/// Type-level `enum` representing a SERCOM pad number
pub trait PadNum: Sealed {}

macro_rules! padnum {
    ( $( $PadNum:ident),+ ) => {
        $(
            /// Represents the corresponding SERCOM pad number
            pub enum $PadNum {}
            impl Sealed for $PadNum {}
            impl PadNum for $PadNum {}
        )+
    };
}

padnum!(Pad0, Pad1, Pad2, Pad3);

//==============================================================================
//  IoSet
//==============================================================================

/// Type-level `enum` representing a SERCOM IOSET configuration
#[cfg(feature = "min-samd51g")]
pub trait IoSet: Sealed {}

#[cfg(feature = "min-samd51g")]
macro_rules! ioset {
    ( $($IoSet:ident),+ ) => {
        $(
            /// Represents the corresponding IOSET
            pub enum $IoSet {}
            impl Sealed for $IoSet {}
            impl IoSet for $IoSet {}
        )+
    };
}

#[cfg(feature = "min-samd51g")]
ioset!(IoSet1, IoSet2, IoSet3, IoSet4);
#[cfg(feature = "min-samd51j")]
ioset!(IoSet5);
#[cfg(feature = "min-samd51g")]
ioset!(IoSet6);

//==============================================================================
//  Four-pad tuple struct
//==============================================================================

/// Tuple struct containing all four [`Pad`]s for a given [`Sercom`] and
/// [`IoSet`]
#[cfg(feature = "min-samd51g")]
pub struct Pads<S, I>(
    pub Pad<S, Pad0, I>,
    pub Pad<S, Pad1, I>,
    pub Pad<S, Pad2, I>,
    pub Pad<S, Pad3, I>,
)
where
    S: Sercom,
    I: IoSet + Map<S, Pad0> + Map<S, Pad1> + Map<S, Pad2> + Map<S, Pad3>;

//==============================================================================
//  Pin-to-pad mapping
//==============================================================================

/// Type-level function mapping [`Pad`]s to [`Pin`]s
///
/// This trait acts as a type-level function. It takes two types as arguments,
/// the [`Sercom`] and [`PadNum`] of a [`Pad`], and returns the [`PinId`] and
/// [`PinMode`] for the corresponding [`Pin`].
///
/// For the SAMD51 and SAME5x series chips, all pins for a given SERCOM must
/// come from the same IOSET. To account for this, we introduce a new
#[cfg_attr(feature = "min-samd51g", doc = "[`IoSet`]")]
#[cfg_attr(not(feature = "min-samd51g"), doc = "`IoSet`")]
/// meta-type and implement [`Map`] on its instances. For a given [`Sercom`] and
/// [`PadNum`], the
#[cfg_attr(feature = "min-samd51g", doc = "[`IoSet`]")]
#[cfg_attr(not(feature = "min-samd51g"), doc = "`IoSet`")]
/// uniquely identifies a corresponding [`PinId`] and
/// [`PinMode`].
///
/// The SAMD11 and SAMD21 series chips are not limited by IOSET. Any combination
/// of valid pins for a given SERCOM is acceptable. Thus, the [`Map`] trait is
/// implemented directly on [`PinId`]s. Because the same [`PinId`] can often be
/// used as a SERCOM [`Pad`] in two different [`PinMode`]s, the [`Map`] trait
/// acts to map a [`Sercom`]/[`PadNum`] pair to the correct [`PinMode`].
pub trait Map<S, P>
where
    S: Sercom,
    P: PadNum,
{
    /// The [`PinId`] for the corresponding pin
    type Id: PinId;
    /// The [`PinMode`] for the corresponding pin
    type Mode: PinMode;
}

//==============================================================================
//  Pad struct
//==============================================================================

/// Represents a SERCOM Pad configured to use a particular pin
///
/// Each [`Pad`] is parameterized by a [`Sercom`], a [`PadNum`], and a third
/// type that implements [`Map`], which is used to determine the corresponding
/// [`Pin`] and its configuration.
///
/// For the SAMD51 and SAME5x chips, [`Map`] is implemented on instances of
#[cfg_attr(feature = "min-samd51g", doc = "[`IoSet`]")]
#[cfg_attr(not(feature = "min-samd51g"), doc = "`IoSet`")]
/// . The SAMD11 and SAMD21 do not have any concept of IOSET, so
/// [`Map`] is instead implemented directly on the corresponding [`PinId`].
///
/// Each [`Pad`] takes ownership of the corresponding [`Pin`] for the duration
/// of its lifetime. [`Pad`]s can be converted to and from [`Pin`]s using the
/// [`Into`] and [`From`] traits.
pub struct Pad<S, P, M>
where
    S: Sercom,
    P: PadNum,
    M: Map<S, P>,
{
    pub(crate) pin: Pin<M::Id, M::Mode>,
}

impl<S, P, M> Pad<S, P, M>
where
    S: Sercom,
    P: PadNum,
    M: Map<S, P>,
{
    /// Create a new SERCOM [`Pad`] from a [`Pin`]
    ///
    /// The specified [`Map`] type must map the specified [`Sercom`] and
    /// [`PadNum`] to the given [`Pin`]
    #[inline]
    pub fn new<O: PinMode>(pin: Pin<M::Id, O>) -> Self
    where
        Pin<M::Id, O>: Into<Pin<M::Id, M::Mode>>,
    {
        Pad { pin: pin.into() }
    }

    /// Consume the [`Pad`] and release the corresponding [`Pin`]
    #[inline]
    pub fn free(self) -> Pin<M::Id, M::Mode> {
        self.pin
    }
}

impl<S, P, M> Sealed for Pad<S, P, M>
where
    S: Sercom,
    P: PadNum,
    M: Map<S, P>,
{
}

//==============================================================================
//  AnyPad
//==============================================================================

/// Meta-type representing any [`Pad`]
///
/// All instances of [`Pad`] implement this trait. When used as a trait bound,
/// it acts to encapsulate a [`Pad`]. Without this trait, a completely generic
/// [`Pad`] requires three type parameters, i.e. `Pad<S, P, M>`. But when using
/// this trait, only one type parameter is required, i.e. `P: AnyPad`. However,
/// even though we have dropped type parameters, no information is lost, because
/// the [`Sercom`], [`PadNum`] and [`Map`] type parameters are stored as
/// associated types in the trait. The implementation of [`AnyPad`] looks like
/// this:
///
/// ```
/// impl<S: Sercom, P: PadNum, M: Map<S, P>> AnyPad for Pad<S, P, M> {
///     type Sercom = S;
///     type PadNum = P;
///     type Map = M;
/// }
/// ```
///
/// Thus, there is a one-to-one mapping between `Pad<S, P, M>` and
/// `AnyPad<Sercom = S, PadNum = P, Map = M>`, so you can always recover the
/// full, concrete type from an implementation of [`AnyPad`]. The type alias
/// [`SpecificPad`] is provided for this purpose. You can convert between
/// [`AnyPad`] and its corresponding [`SpecificPad`] using the [`Into`],
/// [`AsRef`] and [`AsMut`] traits.
///
/// ```
/// fn example<P: AnyPad>(mut any_pad: P) {
///     let pad_mut: &mut SpecificPad<P> = any_pad.as_mut();
///     let pad_ref: &SpecificPad<P> = any_pad.as_ref();
///     let pad: SpecificPad<P> = any_pad.into();
/// }
/// ```
///
/// ## `AnyPad` as a trait bound
///
/// When using [`AnyPad`] as a trait bound, you can constrain the associated
/// types to restrict the acceptable [`Pad`]s. For example, you could restrict
/// a function to accept a particular pad number.
///
/// ```
/// fn example<P>(pad: P)
/// where
///     P: AnyPad<PadNum = Pad2>
/// {
/// }
/// ```
///
/// Or you could accept any pad number, as long as it's in the desired SERCOM.
///
/// ```
/// fn example<P>(pad: P)
/// where
///     P: AnyPad<Sercom = Sercom4>
/// {
/// }
/// ```
///
/// You can also apply more complex bounds. In this case, the [`PadNum`] must
/// satisfy some `UserTrait`.
///
/// ```
/// fn example<P>(pad: P)
/// where
///     P: AnyPad,
///     P::PadNum: UserTrait,
/// {
/// }
/// ```
///
/// ## Generic `AnyPad`s
///
/// Working with a generic type constrained by [`AnyPad`] is slightly different
/// than working with a [`Pad`] directly. When compiling a generic function, the
/// compiler cannot assume anything about the specific concrete type. It can
/// only use what it knows about the [`AnyPad`] trait. To use a generic
/// [`AnyPad`], you must first convert it to its corresponding [`SpecificPad`]
/// using the [`Into`], [`AsRef`] or [`AsMut`] trait. In some instances, you may
/// also need to convert back.
///
/// The documentation for [`AnyPin`](crate::gpio::v2::AnyPin) provides a more
/// detailed example.
pub trait AnyPad: Sealed + Is<Type = SpecificPad<Self>> {
    type Sercom: Sercom;
    type PadNum: PadNum;
    type Map: Map<Self::Sercom, Self::PadNum>;
}

impl<S, P, M> AnyPad for Pad<S, P, M>
where
    S: Sercom,
    P: PadNum,
    M: Map<S, P>,
{
    type Sercom = S;
    type PadNum = P;
    type Map = M;
}

/// Type alias to recover the specific [`Pad`] from an implementation of
/// [`AnyPad`]
pub type SpecificPad<P> = Pad<<P as AnyPad>::Sercom, <P as AnyPad>::PadNum, <P as AnyPad>::Map>;

/// Implementation required to satisfy the `Is<Type = SpecificPad<Self>>` bound
/// on [`AnyPad`]
impl<P: AnyPad> AsRef<P> for SpecificPad<P> {
    #[inline]
    fn as_ref(&self) -> &P {
        // SAFETY: This is guaranteed to be safe, because P == SpecificPad<P>
        unsafe { transmute(self) }
    }
}

/// Implementation required to satisfy the `Is<Type = SpecificPad<Self>>` bound
/// on [`AnyPad`]
impl<P: AnyPad> AsMut<P> for SpecificPad<P> {
    #[inline]
    fn as_mut(&mut self) -> &mut P {
        // SAFETY: This is guaranteed to be safe, because P == SpecificPad<P>
        unsafe { transmute(self) }
    }
}

//==============================================================================
//  Optional pads
//==============================================================================

/// Meta-type representing an optional [`Pad`].
///
/// This trait is implemented for every [`Pad`], as well as for [`NoneT`].
pub trait OptionalPad {}
impl OptionalPad for NoneT {}
impl<P: AnyPad> OptionalPad for P {}

/// Meta-type representing a valid [`Pad`].
///
/// When used as a bound, this trait allows you to exclude [`NoneT`] and limit
/// the type to valid [`Pad`]s.
pub trait SomePad: OptionalPad + AnyPad {}
impl<P: AnyPad> SomePad for P {}

//==============================================================================
//  Convert between pin and pad
//==============================================================================

impl<S, P, M> From<Pad<S, P, M>> for Pin<M::Id, M::Mode>
where
    S: Sercom,
    P: PadNum,
    M: Map<S, P>,
{
    /// Convert from a [`Pad`] to its corresponding [`Pin`].
    ///
    /// This transformation is unique for a given [`Pad`].
    #[inline]
    fn from(pad: Pad<S, P, M>) -> Self {
        pad.pin
    }
}

impl<S, P, M, O> From<Pin<M::Id, O>> for Pad<S, P, M>
where
    S: Sercom,
    P: PadNum,
    M: Map<S, P>,
    O: PinMode,
    Pin<M::Id, O>: Into<Pin<M::Id, M::Mode>>,
{
    /// Convert from a [`Pin`] to its corresponding [`Pad`].
    ///
    /// This conversion is not necessarily unique for a given [`Pin`]
    #[inline]
    fn from(pin: Pin<M::Id, O>) -> Self {
        Pad::new(pin)
    }
}
