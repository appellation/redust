pub trait AsBorrowed<'a> {
	type Target: 'a;

	fn as_borrowed(&'a self) -> Self::Target;
}
