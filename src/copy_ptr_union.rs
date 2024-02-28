use super::*;

#[repr(transparent)]
pub struct CopyPtrUnion<Tags> {
    raw: RawPtrUnion<Tags>,
}

impl<Tags> Copy for CopyPtrUnion<Tags> {}
impl<Tags> Clone for CopyPtrUnion<Tags> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Tags: PtrList> CopyPtrUnion<Tags> {
    pub fn new<P: ErasablePtr + Copy, N: Peano>(ptr: P) -> Self
    where
        Tags: Access<P, N>,
    {
        Self {
            raw: RawPtrUnion::from_raw(P::into_raw(ptr), N::VALUE),
        }
    }

    pub fn set<T, N>(&mut self, value: T)
    where
        T: ErasablePtr + Copy,
        N: Peano,
        Tags: Access<T, N>,
    {
        *self = Self::new(value);
    }

    pub fn tag(&self) -> usize {
        self.raw.split().1 as usize
    }

    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        RawPtrUnion::ptr_eq(&this.raw, &other.raw)
    }

    pub fn ptr_hash<S: core::hash::Hasher>(this: &Self, state: &mut S) {
        RawPtrUnion::ptr_hash(&this.raw, state)
    }

    pub fn map_any<F: MapperOutput>(self, f: F) -> F::Output
    where
        Tags: Map<F>,
    {
        unsafe { self.raw.map_any(f) }
    }

    pub fn is<P: ErasablePtr, N: Peano>(&self) -> bool
    where
        Tags: Access<P, N>,
    {
        self.raw.split().1 == N::VALUE
    }

    pub fn try_cast<P: ErasablePtr, N: Peano>(&self) -> Option<P>
    where
        Tags: Access<P, N>,
    {
        if self.is::<P, N>() {
            Some(unsafe { P::from_raw(self.raw.split().0) })
        } else {
            None
        }
    }

    pub fn unpack(&self) -> Tags {
        let (ptr, tag) = self.raw.split();
        unsafe { Tags::into_inner(ptr, tag) }
    }

    pub fn try_to_superset<NewTags, Ns>(self) -> Result<CopyPtrUnion<NewTags>, InvalidAlignment>
    where
        Tags: SubsetOf<NewTags, Ns>,
        NewTags: PtrList,
    {
        self.raw.try_to_superset().map(|raw| CopyPtrUnion { raw })
    }

    pub fn to_superset<NewTags, Ns>(self) -> CopyPtrUnion<NewTags>
    where
        Tags: SubsetOf<NewTags, Ns>,
        NewTags: PtrList,
    {
        let raw = self.raw.to_superset();
        CopyPtrUnion { raw }
    }

    pub fn reorganize<NewTags, Ns>(self) -> CopyPtrUnion<NewTags>
    where
        Tags: SubsetOf<NewTags, Ns, Remaining = Nil>,
        NewTags: PtrList,
    {
        let raw = self.raw.reorganize();
        CopyPtrUnion { raw }
    }

    pub fn try_to_subset<NewTags, Ns>(
        self,
    ) -> Result<CopyPtrUnion<NewTags>, CopyPtrUnion<NewTags::Remaining>>
    where
        NewTags: SubsetOf<Tags, Ns>,
    {
        match self.raw.try_to_subset() {
            Ok(raw) => Ok(CopyPtrUnion { raw }),
            Err(raw) => Err(CopyPtrUnion { raw }),
        }
    }

    pub fn take<T: ErasablePtr, N: Peano>(self) -> Result<T, CopyPtrUnion<Tags::Remaining>>
    where
        Tags: Access<T, N>,
    {
        self.try_to_subset::<TypeList![T], TypeList![N]>()
            .map(CopyPtrUnion::into_inner)
    }
}

impl<T: ErasablePtr> CopyPtrUnion<TypeList![T]> {
    pub fn into_inner(self) -> T {
        unsafe { self.raw.into_inner() }
    }
}

impl CopyPtrUnion<TypeList![]> {
    pub fn unreachable(self) -> ! {
        self.raw.unreachable()
    }
}

impl<Tags> Eq for CopyPtrUnion<Tags> where Tags: PtrList + Map<EqAny> + Map<PartialEqAny> {}
impl<Tags> PartialEq for CopyPtrUnion<Tags>
where
    Tags: PtrList + Map<PartialEqAny>,
{
    fn eq(&self, other: &Self) -> bool {
        let (ptr, tag) = other.raw.split();
        self.raw.split().1 == tag && self.map_any(PartialEqAny(ptr))
    }
}

impl<Tags> PartialOrd for CopyPtrUnion<Tags>
where
    Tags: PtrList + Map<PartialEqAny> + Map<PartialOrdAny>,
{
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        let (ptr, tag) = other.raw.split();

        match self.raw.split().1.cmp(&tag) {
            o @ (std::cmp::Ordering::Less | std::cmp::Ordering::Greater) => Some(o),
            std::cmp::Ordering::Equal => self.map_any(PartialOrdAny(ptr)),
        }
    }
}

impl<Tags> Ord for CopyPtrUnion<Tags>
where
    Tags: PtrList + Map<PartialEqAny> + Map<PartialOrdAny> + Map<EqAny> + Map<OrdAny>,
{
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        let (ptr, tag) = other.raw.split();

        match self.raw.split().1.cmp(&tag) {
            o @ (std::cmp::Ordering::Less | std::cmp::Ordering::Greater) => o,
            std::cmp::Ordering::Equal => self.map_any(OrdAny(ptr)),
        }
    }
}

impl<Tags> core::hash::Hash for CopyPtrUnion<Tags>
where
    Tags: PtrList + MapHash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        unsafe { self.raw.map_hash(state) }
    }
}
