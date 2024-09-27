use std::convert::Infallible;
use std::fmt::Debug;
use std::ops::Range;
use std::ops::RangeBounds;
use std::ops::RangeFrom;
use std::ops::RangeFull;
use std::ops::RangeInclusive;
use std::ops::RangeTo;
use std::ops::RangeToInclusive;

pub trait TryIntoRangeBounds<B> {
    type TargetRange: RangeBounds<B>;
    type Error: Debug;

    fn try_into_range_bounds(self) -> Result<Self::TargetRange, Self::Error>;
}

impl<A, B> TryIntoRangeBounds<B> for Range<A>
where
    A: TryInto<B>,
    A::Error: Debug,
{
    type TargetRange = Range<B>;
    type Error = <A as TryInto<B>>::Error;

    fn try_into_range_bounds(self) -> Result<Self::TargetRange, Self::Error> {
        Ok(self.start.try_into()?..self.end.try_into()?)
    }
}

impl<A, B> TryIntoRangeBounds<B> for RangeFrom<A>
where
    A: TryInto<B>,
    A::Error: Debug,
{
    type TargetRange = RangeFrom<B>;
    type Error = <A as TryInto<B>>::Error;

    fn try_into_range_bounds(self) -> Result<Self::TargetRange, Self::Error> {
        Ok(self.start.try_into()?..)
    }
}

impl<A, B> TryIntoRangeBounds<B> for RangeTo<A>
where
    A: TryInto<B>,
    A::Error: Debug,
{
    type TargetRange = RangeTo<B>;
    type Error = <A as TryInto<B>>::Error;

    fn try_into_range_bounds(self) -> Result<Self::TargetRange, Self::Error> {
        Ok(..self.end.try_into()?)
    }
}

impl<A, B> TryIntoRangeBounds<B> for RangeInclusive<A>
where
    A: TryInto<B>,
    A::Error: Debug,
{
    type TargetRange = RangeInclusive<B>;
    type Error = <A as TryInto<B>>::Error;

    fn try_into_range_bounds(self) -> Result<Self::TargetRange, Self::Error> {
        let (start, end) = self.into_inner();
        Ok(start.try_into()?..=end.try_into()?)
    }
}

impl<A, B> TryIntoRangeBounds<B> for RangeToInclusive<A>
where
    A: TryInto<B>,
    A::Error: Debug,
{
    type TargetRange = RangeToInclusive<B>;
    type Error = <A as TryInto<B>>::Error;

    fn try_into_range_bounds(self) -> Result<Self::TargetRange, Self::Error> {
        Ok(..=self.end.try_into()?)
    }
}

impl<B> TryIntoRangeBounds<B> for RangeFull {
    type TargetRange = RangeFull;
    type Error = Infallible;

    fn try_into_range_bounds(self) -> Result<Self::TargetRange, Self::Error> {
        Ok(self)
    }
}
