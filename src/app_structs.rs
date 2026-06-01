//#[derive(Default)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    #[default]
    Base,
    Cards,
    Review,
    NewCard,
    EditCard,
    Quiz,
    Tutors,
    CreateTutor,
    TutorDetail,
    TutorSession,
}
