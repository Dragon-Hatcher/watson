import Mathlib.Tactic
import Batteries.Util.ExtendedBinder

namespace Watson

universe u

def Set (α : Type u) := α → Prop

def setOf {α : Type u} (p: α → Prop) : Set α := p

syntax myBinder := Lean.binderIdent (" : " term)?
syntax (name := wSetBuilder) (priority := high) "{ " myBinder  " | " term " }" : term

@[term_elab wSetBuilder]
def elabWSetBuilder : Lean.Elab.Term.TermElab
  | `({ $x:ident | $p }), expectedType? => do
      Lean.Elab.Term.elabTerm (← `(setOf fun $x:ident ↦ $p)) expectedType?
  | `({ $x:ident : $t | $p }), expectedType? => do
      Lean.Elab.Term.elabTerm (← `(setOf fun ($x:ident : $t) ↦ $p)) expectedType?
  | _, _ => Lean.Elab.throwUnsupportedSyntax

@[app_unexpander setOf]
def setOf.unexpander : Lean.PrettyPrinter.Unexpander
  | `($_ fun $x:ident ↦ $p) => `({ $x:ident | $p })
  | `($_ fun ($x:ident : $ty:term) ↦ $p) => `({ $x:ident : $ty:term | $p })
  | _ => throw ()

macro "{" t:term " | " b:Batteries.ExtendedBinder.extBinders "}" : term =>
  `({x | ∃ᵉ $b:extBinders, $t = x})

namespace Set

variable {α β : Type u}

def mem (s : Set α) (a : α) : Prop := s a

instance : Membership α (Set α) where
  mem := mem


theorem ext {a b : Set α} (h : ∀ (x : α), x ∈ a ↔ x ∈ b) : a = b := by
  apply funext
  intro x
  exact propext (h x)


def empty : Set α := { _a | False }

instance : EmptyCollection (Set α) where
  emptyCollection := empty

theorem not_in_empty : ∀ (x : α), x ∉ empty := by
  intro x
  show ¬False
  simp

def univ : Set α := { _a | True }

theorem in_univ : ∀ (x : α), x ∈ univ := by
  intro x
  show True
  simp


def non_empty (s : Set α) := ∃ x, x ∈ s

theorem non_empty_iff_ne_empty (s : Set α) : s.non_empty ↔ s ≠ ∅ := by
  constructor
  · intro ⟨x, hx⟩
    by_contra s_empty
    rw [s_empty] at hx
    have hnx : x ∉ ∅ := not_in_empty x
    contradiction
  intro s_ne_empty
  by_contra hs
  have hns : s = ∅ := by
    apply ext
    intro x
    have xns      : x ∉ s := (not_exists.mp hs) x
    have xn_empty : x ∉ ∅ := not_in_empty x
    have iff : _ ↔ _ := Iff.not ⟨fun _ ↦ xns, fun _ ↦ xn_empty⟩
    simp at iff
    exact iff.symm
  contradiction


protected def singleton (a : α) := { b | b = a }

instance : Singleton α (Set α) where
  singleton := Set.singleton


def sep (motive : α → Prop) (s : Set α) : Set α :=
  {x | x ∈ s ∧ motive x}

instance : Sep α (Set α) where
  sep := sep

protected def union (s₁ s₂ : Set α) := { a | a ∈ s₁ ∨ a ∈ s₂ }

instance : Union (Set α) where
  union := Set.union

protected def insert (a : α) (s : Set α) := { a } ∪ s

instance : Insert α (Set α) where
  insert := Set.insert

protected def inter (s₁ s₂ : Set α) := { a | a ∈ s₁ ∧ a ∈ s₂ }

instance : Inter (Set α) where
  inter := Set.inter

protected def complement (s : Set α) := { a | a ∉ s }

instance : HasCompl (Set α) where
  compl := Set.complement

protected def diff (s₁ s₂ : Set α) := { a | a ∈ s₁ ∧  a ∉ s₂ }

instance : SDiff (Set α) where
  sdiff := Set.diff


theorem union_comm (a b : Set α) : a ∪ b = b ∪ a := by
  apply ext
  intro x
  exact or_comm

theorem union_assoc (a b c : Set α) : (a ∪ b) ∪ c = a ∪ (b ∪ c) := by
  apply ext
  intro x
  exact or_assoc

theorem inter_comm (a b : Set α) : a ∩ b = b ∩ a := by
  apply ext
  intro x
  exact and_comm

theorem inter_assoc (a b c : Set α) : (a ∩ b) ∩ c = a ∩ (b ∩ c) := by
  apply ext
  intro x
  exact and_assoc

@[simp]
theorem diff_union_compl (a : Set α) : univ \ a = aᶜ := by
  apply ext
  intro x
  show True ∧ x ∉ a ↔ x ∉ a
  simp

@[simp]
theorem not_in_compl (s : Set α) (x : α) : x ∉ sᶜ ↔ x ∈ s := by
  show ¬¬(x ∈ s) ↔ x ∈ s
  exact not_not

@[simp]
theorem union_empty (a : Set α) : a ∪ ∅ = a := by
  apply ext
  intro x
  show x ∈ a ∨ False ↔ x ∈ a
  simp

@[simp]
theorem union_self (a : Set α) : a ∪ a = a := by
  apply ext
  intro x
  show x ∈ a ∨ x ∈ a ↔ x ∈ a
  simp

@[simp]
theorem inter_self (a : Set α) : a ∩ a = a := by
  apply ext
  intro x
  show x ∈ a ∧ x ∈ a ↔ x ∈ a
  simp

@[simp]
theorem empty_union (a : Set α) : ∅ ∪ a = a := by simp [union_comm]

@[simp]
theorem inter_empty (a : Set α) : a ∩ ∅ = ∅ := by
  apply ext
  intro x
  show x ∈ a ∧ False ↔ False
  simp

@[simp]
theorem empty_inter (a : Set α) : ∅ ∩ a = ∅ := by simp [inter_comm]

@[simp]
theorem union_univ (a : Set α) : a ∪ univ = univ := by
  apply ext
  intro x
  show x ∈ a ∨ True ↔ True
  simp

@[simp]
theorem univ_union (a : Set α) : univ ∪ a = univ := by simp [union_comm]

@[simp]
theorem inter_univ (a : Set α) : a ∩ univ = a := by
  apply ext
  intro x
  show x ∈ a ∧ True ↔ x ∈ a
  simp

@[simp]
theorem univ_inter (a : Set α) : univ ∩ a = a := by simp [inter_comm]

theorem inter_union_distrib_left (a b c : Set α)
  : a ∩ (b ∪ c) = (a ∩ b) ∪ (a ∩ c) := ext (fun _ ↦ and_or_left)

theorem union_inter_distrib_right (a b c : Set α)
  : (b ∪ c) ∩ a = (b ∩ a) ∪ (c ∩ a) := ext (fun _ ↦ or_and_right)

theorem union_inter_distrib_left (a b c : Set α)
  : a ∪ (b ∩ c) = (a ∪ b) ∩ (a ∪ c) := ext (fun _ ↦ or_and_left)

theorem inter_union_distrib_right (a b c : Set α)
  : (b ∩ c) ∪ a = (b ∪ a) ∩ (c ∪ a) := ext (fun _ ↦ and_or_right)

theorem compl_eq_univ_diff (a : Set α) : aᶜ = univ \ a := by
  apply ext
  intro x
  show x ∉ a ↔ True ∧ x ∉ a
  simp

@[simp]
theorem self_union_compl (a : Set α) : a ∪ aᶜ = univ :=
  ext fun _ ↦ Eq.mpr (iff_true _) (em _)

@[simp]
theorem self_inter_compl (a : Set α) : a ∩ aᶜ = ∅ :=
  ext fun _ ↦ and_not_self_iff _

@[simp]
theorem compl_compl (s : Set α) : sᶜᶜ = s :=
  ext fun _ ↦ not_not

theorem compl_eq_iff_eq_compl (a b : Set α) : aᶜ = b ↔ a = bᶜ := by
  constructor <;> intro h
  · simp [← h]
  · simp [h]

@[simp]
theorem compl_empty : (∅ : Set α)ᶜ = univ :=
  ext fun _ ↦ Iff.of_eq not_false_eq_true

@[simp]
theorem compl_univ : univ ᶜ = (∅ : Set α) :=
  Eq.symm ((compl_eq_iff_eq_compl _ _).mp compl_empty)

@[simp]
theorem compl_union (a b : Set α) : (a ∪ b)ᶜ = aᶜ ∩ bᶜ :=
  ext fun _ ↦ not_or

theorem compl_inter (a b : Set α) : (a ∩ b)ᶜ = aᶜ ∪ bᶜ :=
  ext fun _ ↦ Classical.not_and_iff_or_not_not



def disjoint (a b : Set α) := a ∩ b = ∅

theorem diff_disjoint_inter {a b : Set α} : Set.disjoint (a \ b) (a ∩ b) := by
  apply ext
  intro x
  simp [(Iff.of_eq (iff_false (x ∈ ∅))).mpr (not_in_empty x)]
  intro ⟨⟨_, hnb⟩, ⟨_, hb⟩⟩
  contradiction

theorem diff_disjoint_diff {a b : Set α} : Set.disjoint (a \ b) (b \ a) := by
  apply ext
  intro x
  simp [(Iff.of_eq (iff_false (x ∈ ∅))).mpr (not_in_empty x)]
  intro ⟨⟨ha, _⟩, ⟨_, hna⟩⟩
  contradiction

protected def subset (a b : Set α) := ∀ x ∈ a, x ∈ b

instance : HasSubset (Set α) where
  Subset := Set.subset

protected def ssubset (a b : Set α) := (∀ x ∈ a, x ∈ b) ∧ a ≠ b

instance : HasSSubset (Set α) where
  SSubset := Set.ssubset

theorem ssub_sub {a b : Set α} : a ⊂ b → a ⊆ b := by
  intro h
  exact h.left

theorem sub_rfl (a : Set α) : a ⊆ a := by
  show ∀ x ∈ a, x ∈ a
  simp

@[simp]
theorem empty_sub (a : Set α) : ∅ ⊆ a := by
  intro x x_in_empty
  have _ := not_in_empty x
  contradiction


@[simp]
theorem sub_trans {a b c : Set α} (hab : a ⊆ b) (hbc : b ⊆ c) : a ⊆ c := by
  intro x x_in_a
  exact hbc x (hab x x_in_a)

theorem sub_anti_rfl {a b : Set α} (hab : a ⊆ b) (hba : b ⊆ a) : a = b := by
  apply ext
  intro x
  exact ⟨hab x, hba x⟩

theorem ssub_asymm {a b : Set α} : a ⊂ b → ¬b ⊂ a := by
  intro ⟨hab, a_ne_b⟩ ⟨hba, _⟩
  have a_eq_b : a = b := by
    apply ext
    intro x
    exact ⟨hab x, hba x⟩
  contradiction

theorem ssub_trans {a b c : Set α} (hab : a ⊂ b) (hbc : b ⊂ c) : a ⊂ c := by
  constructor
  · exact sub_trans (ssub_sub hab) (ssub_sub hbc)
  intro a_eq_c
  rw [a_eq_c] at hab
  have hn_bc := ssub_asymm hab
  contradiction

@[simp]
theorem union_eq_right {a b : Set α} : a ∪ b = b ↔ a ⊆ b := by
  constructor
  · intro h x ha
    rw [← h]
    exact Or.inl ha
  intro (h : ∀ x, x ∈ a → x ∈ b)
  apply ext
  intro x
  constructor
  · intro hab
    cases hab with
    | inl ha => exact h x ha
    | inr hb => assumption
  intro hb
  exact Or.inr hb

@[simp]
theorem union_eq_left {a b : Set α} : a ∪ b = a ↔ b ⊆ a := by
  rw [union_comm]
  exact union_eq_right

@[simp]
theorem inter_eq_left {a b : Set α} : a ∩ b = a ↔ a ⊆ b := by
  constructor
  · intro h x ha
    rw [← h] at ha
    exact ha.right
  intro (h : ∀ x, x ∈ a → x ∈ b)
  apply ext
  intro x
  constructor
  · intro hab
    exact hab.left
  intro ha
  exact ⟨ha, h x ha⟩

@[simp]
theorem inter_eq_right {a b : Set α} : a ∩ b = b ↔ b ⊆ a := by
  rw [inter_comm]
  exact inter_eq_left

@[simp]
theorem left_sub_union {a b : Set α} : a ⊆ a ∪ b := by
  intro x ha
  exact Or.inl ha

@[simp]
theorem right_sub_union {a b : Set α} : b ⊆ a ∪ b := by
  intro x hb
  exact Or.inr hb

@[simp]
theorem inter_sub_left {a b : Set α} : a ∩ b ⊆ a := by
  intro x hab
  exact hab.left

@[simp]
theorem inter_sub_right {a b : Set α} : a ∩ b ⊆ b := by
  intro x hab
  exact hab.right

@[simp]
theorem union_sub_iff {a b c : Set α} : a ∪ b ⊆ c ↔ a ⊆ c ∧ b ⊆ c := by
  constructor
  · intro h
    constructor
    · intro x ha; exact h x (Or.inl ha)
    · intro x hb; exact h x (Or.inr hb)
  intro ⟨ha, hb⟩ x hab
  cases hab with
  | inl hxa => exact ha x hxa
  | inr hxb => exact hb x hxb

@[simp]
theorem sub_inter_iff {a b c : Set α} : c ⊆ a ∩ b ↔ c ⊆ a ∧ c ⊆ b := by
  constructor
  · intro hab
    constructor
    · intro x hc; exact (hab x hc).left
    · intro x hc; exact (hab x hc).right
  intro ⟨ha, hb⟩ x hc
  exact ⟨ha x hc, hb x hc⟩

@[simp]
theorem inter_union_self {a b : Set α} : a ∩ (a ∪ b) = a := by
  apply ext
  intro x
  constructor
  · intro h; exact h.left
  · intro ha; exact ⟨ha, Or.inl ha⟩

@[simp]
theorem union_inter_self {a b : Set α} : a ∪ (a ∩ b) = a := by
  apply ext
  intro x
  constructor
  · intro h;
    cases h with
    | inl _   => assumption
    | inr hab => exact hab.left
  · intro ha; exact Or.inl ha

theorem union_univ_inter_empty {a b : Set α}
  (h₁ : a ∪ b = Set.univ) (h₂ : a ∩ b = ∅) : a = bᶜ := by
  apply ext
  intro x
  constructor
  · intro ha
    by_contra hb
    simp at hb
    have hab : x ∈ a ∩ b := ⟨ha, hb⟩
    rw [h₂] at hab
    exact (not_in_empty x) hab
  intro hbc
  have hbn := (not_in_compl _ _).mpr hbc
  simp at hbn
  by_contra han
  have hn : x ∉ a ∪ b := not_or_intro han hbn
  rw [h₁] at hn
  exact hn (in_univ x)


end Set

end Watson
