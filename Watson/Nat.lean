import Mathlib.Tactic

namespace Watson

open _root_ renaming Nat → LeanNat

/--
The integers greater than or equal to zero. Defined in terms of zero and a
successor function.
-/
inductive Nat where
  | zero : Nat
  | succ (n : Nat) : Nat

def Nat.one : Nat := succ zero

/-! Test -/
-- Here we make it so we can use integer literals for instances of our custom
-- `Nat` type.

def ofNat : (n : LeanNat) → Nat
  | LeanNat.zero   => Nat.zero
  | LeanNat.succ m => Nat.succ (ofNat m)

instance (n : LeanNat) : OfNat Nat n where
  ofNat := ofNat n

@[simp]
theorem Nat.zero_eq_lit : Nat.zero = 0 := rfl


theorem Nat.succ_ne_zero {n : Nat} : n.succ ≠ 0 :=
  Nat.noConfusion

theorem Nat.succ_inj {a b : Nat} : a.succ = b.succ ↔ a = b :=
  (Nat.succ.injEq a b).to_iff


def Nat.add : (a b : Nat) → Nat
  | a, zero    => a
  | a, succ b' => Nat.succ (Nat.add a b')

instance : Add Nat where
  add := Nat.add

@[simp]
theorem Nat.add_zero (n : Nat) : n + 0 = n := rfl

@[simp]
theorem Nat.succ_eq_add_one (n : Nat) : n.succ = n + 1 := rfl

@[simp]
theorem Nat.add_succ (n m : Nat) : n + m.succ = (n + m).succ := rfl

@[simp]
theorem Nat.zero_add (n : Nat) : 0 + n = n := by
  induction n with
  | zero => rfl
  | succ n' ih => rw [add_succ, ih]

@[simp]
theorem Nat.succ_add (n m : Nat) : n.succ + m = (n + m).succ := by
  induction m with
  | zero => simp
  | succ n' ih => rw [add_succ, ih]; rfl

theorem Nat.add_comm (n m : Nat) : n + m = m + n := by
  induction n with
  | zero => simp
  | succ n' ih => rw [add_succ, succ_add, ih]

theorem Nat.add_assoc (a b c : Nat) : (a + b) + c = a + (b + c) := by
  induction c with
  | zero => rw [zero_eq_lit, add_zero, add_zero]
  | succ n' ih => rw [add_succ, add_succ, add_succ, ih]

@[simp]
theorem Nat.add_cancels {a b c : Nat} : a + b = a + c → b = c := by
  induction a with
  | zero => simp
  | succ a' ih =>
      rw [succ_add, succ_add, succ_inj]
      assumption

theorem Nat.ne_add_one (n : Nat) : n ≠ n + 1 := by
  induction n with
  | zero => exact succ_ne_zero.symm
  | succ n' =>
      intro h
      have hh := succ_inj.mp h
      contradiction

theorem Nat.add_one_ne (n : Nat) : n + 1 ≠ n := (ne_add_one n).symm


def Nat.is_pos (n : Nat) := n ≠ 0

theorem Nat.pos_add_is_pos (a b : Nat) : a.is_pos → (a + b).is_pos := by
  intro ha
  induction b with
  | zero => simp; assumption
  | succ b' ih =>
      simp
      exact Nat.succ_ne_zero

theorem Nat.sum_zero_not_pos {a b : Nat} : a + b = 0 → a = 0 ∧ b = 0 := by
  intro h
  constructor
  . by_contra! ha
    have ab_pos := Nat.pos_add_is_pos a b ha
    contradiction
  by_contra! ha
  rw [add_comm] at h
  have ab_pos := Nat.pos_add_is_pos b a ha
  contradiction

theorem Nat.exists_pred (a : Nat) (h : a.is_pos) : ∃ b : Nat, b.succ = a := by
  induction a with
  | zero => contradiction
  | succ a' ih => use a'

theorem Nat.ne_add_pos {a b : Nat} (hb : b.is_pos) : a ≠ a + b := by
  intro h
  nth_rw 1 [← add_zero a] at h
  have hbn := (add_cancels h).symm
  contradiction


def Nat.le (n m : Nat) := ∃ a, m = n + a
def Nat.lt (n m : Nat) := ∃ a, m = n + a ∧ a.is_pos

instance : LE Nat where
  le := Nat.le

instance : LT Nat where
  lt := Nat.lt

theorem Nat.not_lt_zero (n : Nat) : ¬(n < 0) := by
  intro ⟨diff, ⟨h_diff, diff_pos⟩⟩
  have _ : diff = 0 := (sum_zero_not_pos h_diff.symm).right
  contradiction

@[simp]
theorem Nat.zero_le (n : Nat) : 0 ≤ n := ⟨n, (zero_add n).symm⟩

theorem Nat.le_zero_eq_zero (n : Nat) : n ≤ 0 → n = 0 := by
  intro ⟨diff, h_diff⟩
  exact (sum_zero_not_pos h_diff.symm).left

theorem Nat.pos_iff_gt_zero {n : Nat} : n.is_pos ↔ 0 < n := by
  constructor
  · intro pos
    exact ⟨n, ⟨by simp, pos⟩⟩
  intro ⟨diff, ⟨h_diff, diff_pos⟩⟩
  simp at h_diff
  rw [h_diff]
  assumption

theorem Nat.lt_is_le {a b : Nat} : a < b → a ≤ b := by
  intro ⟨n, ⟨h, _⟩⟩
  exact ⟨n, h⟩

theorem Nat.le_rfl (a : Nat) : a ≤ a :=
  ⟨0, by simp⟩

theorem Nat.le_trans {a b c : Nat} (hab : a ≤ b) (hbc : b ≤ c) : a ≤ c := by
  have ⟨b_min_a, h_b_min_a⟩ := hab
  have ⟨c_min_b, h_c_min_b⟩ := hbc
  rw [h_b_min_a, add_assoc] at h_c_min_b
  use b_min_a + c_min_b

theorem Nat.lt_le_is_lt {a b c : Nat} (hab : a < b) (hbc : b ≤ c) : a < c := by
  have ⟨b_min_a, ⟨h_b_min_a, b_min_a_pos⟩⟩ := hab
  have ⟨c_min_b, h_c_min_b⟩ := hbc
  rw [h_b_min_a, add_assoc] at h_c_min_b
  use b_min_a + c_min_b
  exact ⟨h_c_min_b, pos_add_is_pos _ _ b_min_a_pos⟩

theorem Nat.lt_trans {a b c : Nat} (hab : a < b) (hbc : b < c) : a < c := by
  have hbc := lt_is_le hbc
  exact lt_le_is_lt hab hbc

theorem Nat.le_anti_symm {a b : Nat} (hab : a ≤ b) (hba : b ≤ a) : a = b := by
  have ⟨b_min_a, h_b_min_a⟩ := hba
  have ⟨a_min_b, h_a_min_b⟩ := hab
  rw [h_b_min_a, add_assoc] at h_a_min_b
  nth_rw 1 [← add_zero b] at h_a_min_b
  have ⟨hb₀, ha₀⟩ := sum_zero_not_pos (add_cancels h_a_min_b).symm
  rw [h_b_min_a, hb₀, add_zero]

theorem Nat.le_iff_le_add {a b c : Nat} : a ≤ b ↔ a + c ≤ b + c := by
  constructor
  · intro ⟨b_min_a, h_b_min_a⟩
    use b_min_a
    rw [h_b_min_a]
    rw [add_assoc, add_comm b_min_a, ← add_assoc]
  intro ⟨b_min_a, h_b_min_a⟩
  rw [add_comm, add_comm a, add_assoc] at h_b_min_a
  have h_b_min_a := add_cancels h_b_min_a
  use b_min_a

theorem Nat.lt_succ (a : Nat) : a < a.succ := by
  use 1
  exact ⟨rfl, succ_ne_zero⟩

theorem Nat.le_succ (a : Nat) : a ≤ a.succ := lt_is_le (lt_succ a)

theorem Nat.lt_iff_succ_le {a b : Nat} : a < b ↔ a.succ ≤ b := by
  constructor
  · intro ⟨diff, ⟨h_diff, diff_pos⟩⟩
    have ⟨diff', h_diff'⟩ := exists_pred diff diff_pos
    rw [← h_diff', add_succ, ← succ_add] at h_diff
    exact ⟨diff', h_diff⟩
  intro ⟨diff, h_diff⟩
  rw [succ_eq_add_one, add_assoc] at h_diff
  use 1 + diff
  exact ⟨h_diff, by rw [add_comm]; exact succ_ne_zero⟩

theorem Nat.lt_succ_iff_le {a b : Nat} : a < b.succ ↔ a ≤ b := by
  constructor
  · intro ⟨diff, ⟨h_diff, diff_pos⟩⟩
    have ⟨diff', h_diff'⟩ := exists_pred diff diff_pos
    rw [← h_diff', add_succ, succ_inj] at h_diff
    exact ⟨diff', h_diff⟩
  intro ⟨diff, h_diff⟩
  rw [h_diff, ← add_succ]
  use diff.succ
  exact ⟨rfl, succ_ne_zero⟩

theorem Nat.le_lt_or_eq {a b : Nat} (h : a ≤ b) : a < b ∨ a = b := by
  have ⟨diff, h_diff⟩ := h
  cases diff with
  | zero       => right; simp at h_diff; exact h_diff.symm
  | succ diff' => left; exact ⟨diff'.succ, ⟨h_diff, succ_ne_zero⟩⟩

theorem Nat.lt_trichotomy (a b : Nat) : a < b ∨ a = b ∨ a > b := by
  induction a with
  | zero       => cases b with
    | zero     => right; left; rfl
    | succ b'  => left; use b'.succ; exact ⟨by simp, succ_ne_zero⟩
  | succ a' ih =>
      rcases ih with h | h | h
      · have hle : a'.succ ≤ b := lt_iff_succ_le.mp h
        cases le_lt_or_eq hle
        · left; assumption
        · right; left; assumption
      · right; right;
        exact ⟨1, ⟨by rw [h]; rfl, succ_ne_zero⟩⟩
      right; right;
      have ha : a'.succ > a' := lt_succ a'
      exact lt_trans h ha

theorem Nat.lt_ne (a b : Nat) : a < b → a ≠ b := by
  intro ⟨diff, ⟨h_diff, diff_pos⟩⟩
  rw [h_diff]
  nth_rw 1 [← add_zero a]
  intro h
  have diff_zero := (add_cancels h).symm
  contradiction


theorem Nat.strong_induction' (motive : Nat → Prop) (m₀ : Nat)
  (h : ∀ m ≥ m₀, (∀ m', m₀ ≤ m' ∧ m' < m → motive m') → motive m)
  : ∀ m ≥ m₀, motive m := by
  intro m h_m_ge_m₀
  have ha : ∀ n, ∀ m', m₀ ≤ m' ∧ m' < n → motive m' := by
    intro n
    induction n with
    | zero =>
        intro m' ⟨_, m'_lt_0⟩
        absurd m'_lt_0
        exact not_lt_zero m'
    | succ n' ih =>
        intro m' ⟨m₀_le_m', m'_lt_n'_succ⟩
        apply h m' m₀_le_m'
        intro m'' ⟨m₀_le_m'', m''_lt_m'⟩
        apply ih
        have m'_le_n' := lt_succ_iff_le.mp m'_lt_n'_succ
        exact ⟨m₀_le_m'', lt_le_is_lt m''_lt_m' m'_le_n'⟩
  exact h m h_m_ge_m₀ (ha m)

theorem Nat.strong_induction (motive : Nat → Prop)
  (h : ∀ m, (∀ m' < m, motive m') → motive m)
  : ∀ m, motive m := by
  have h' : ∀ m ≥ 0, (∀ m', 0 ≤ m' ∧ m' < m → motive m') → motive m := by
    simp; assumption
  have ha := strong_induction' motive 0 h'
  simp at ha; assumption


def Nat.mul : (a b : Nat) → Nat
  | _, zero    => 0
  | a, succ b' => Nat.mul a b' + a

instance : Mul Nat where
  mul := Nat.mul

@[simp]
theorem Nat.mul_zero (n : Nat) : n * 0 = 0 := rfl

@[simp]
theorem Nat.mul_succ (n m : Nat) : n * m.succ = (n * m) + n := rfl

@[simp]
theorem Nat.zero_mul (n : Nat) : 0 * n = 0 := by
  induction n with
  | zero       => rfl
  | succ n' ih => rw [mul_succ, add_zero, ih]

@[simp]
theorem Nat.succ_mul (n m : Nat) : n.succ * m = (n * m) + m := by
  induction m with
  | zero       => simp
  | succ m' ih =>
      rw [mul_succ, mul_succ, ih]
      rw [add_succ, add_succ]
      rw [add_assoc, add_assoc, add_comm m'] -- ring?

@[simp]
theorem Nat.mul_comm (n m : Nat) : n * m = m * n := by
  induction m with
  | zero       => simp
  | succ m' ih => rw [mul_succ, succ_mul, ih]

theorem Nat.mul_zero_zero (n m : Nat) : n * m = 0 ↔ n = 0 ∨ m = 0 := by
  constructor
  · intro h
    induction m with
    | zero       => simp
    | succ m' ih =>
        left
        by_contra hn_ne_0
        have ⟨n', hn'_ne_0⟩ := exists_pred n hn_ne_0
        rw [← hn'_ne_0, mul_succ, add_succ] at h
        absurd h
        exact succ_ne_zero
  intro h
  cases h with
  | inl hn => rw [hn]; simp
  | inr hm => rw [hm]; simp

theorem Nat.mul_add (a b c : Nat) : a * (b + c) = a * b + a * c := by
  induction c with
  | zero       => simp
  | succ c' ih =>
      rw [add_succ, mul_succ, mul_succ, ih]
      rw [← add_assoc] -- ring?

theorem Nat.add_mul (a b c : Nat) : (b + c) * a = b * a + c * a := by
  rw [mul_comm, mul_comm _ a, mul_comm _ a]
  exact Nat.mul_add a b c

theorem Nat.mul_assoc (a b c : Nat) : (a * b) * c = a * (b * c) := by
  induction c with
  | zero       => simp
  | succ c' ih => rw [mul_succ, mul_succ, ih, mul_add]

theorem Nat.lt_mul_lt (a b c : Nat) (hab : a < b) (c_pos : c.is_pos)
  : a * c < b * c := by
  have ⟨diff, ⟨h_diff, diff_pos⟩⟩ := hab
  rw [h_diff, add_mul]
  have diff_c_pos : (diff * c).is_pos := by
    have q := (mul_zero_zero diff c).mp.mt
    have hn : ¬(diff = 0 ∨ c = 0) := not_or.mpr ⟨diff_pos, c_pos⟩
    exact q hn
  use diff * c

theorem Nat.mul_cancels (a b c : Nat) (h : a * c = b * c) (c_pos : c.is_pos)
  : a = b := by
  rcases (Nat.lt_trichotomy a b) with hab | hab | hab
  · have h_ac_bc : a * c < b * c := lt_mul_lt a b c hab c_pos
    have hn := lt_ne _ _ h_ac_bc
    contradiction
  · assumption
  · have h_bc_ac : b * c < a * c := lt_mul_lt b a c hab c_pos
    have hn := (lt_ne _ _ h_bc_ac).symm
    contradiction


theorem Nat.euclidean_algo (n q : Nat) (q_pos : q.is_pos)
  : ∃ m r : Nat, n = m * q + r ∧ r < q := by
  induction n with
  | zero       =>
      use 0, 0
      simp
      exact pos_iff_gt_zero.mp q_pos
  | succ n' ih =>
      have ⟨m', r', ⟨hn', r'_lt_q⟩⟩ := ih
      have r'_succ_le_q := lt_iff_succ_le.mp r'_lt_q
      have hn' := succ_inj.mpr hn'
      rw [← add_succ] at hn'
      cases le_lt_or_eq r'_succ_le_q with
      | inl r'_succ_lt_q => use m', r'.succ
      | inr r'_succ_eq_q =>
          use m'.succ, 0
          rw [r'_succ_eq_q, ← succ_mul] at hn'
          exact ⟨hn', pos_iff_gt_zero.mp q_pos⟩


def Nat.exp : (a k : Nat) → Nat
  | _, zero    => 1
  | a, succ k' => (Nat.exp a k') * a

instance : Pow Nat Nat where
  pow := Nat.exp

end Watson
