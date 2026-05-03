# Grade Calculator

A graphical grade calculator written in Rust. Tracks weighted grades across multiple subjects, saves each subject to its own JSON file, and lets you add or remove grades at any time.

Grades use the **1.0–5.0 scale** (1.0 = best, 5.0 = worst), common in Czech and German schools.

---

## Features

- **Graphical interface** built with [egui](https://github.com/emilk/egui)
- Per-subject persistence — each subject is saved as a separate `.json` file
- Three weighted grade categories per subject: **A (60%)**, **B (30%)**, **C (10%)**
- Weighted final grade calculated automatically, adjusting for any empty categories
- Grades sorted best to worst automatically within each category
- **Grade predictor** — shows exactly what grade you need next to reach a target final grade
- **Overview screen** — overall GPA across all subjects, sorted best to worst with a progress bar
- **Export to CSV** — one click to export all subjects and grades to a spreadsheet
- Press **Enter** to confirm adding a grade

---

## Requirements

Rust must be installed. Get it at [rustup.rs](https://rustup.rs) or run:

**macOS/Linux:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Windows:**
```powershell
winget install Rustlang.Rustup
```

---

## Installation

```bash
git clone https://github.com/Jezek1/kalkulacka_prumeru.git
cd kalkulacka_prumeru
cargo build --release
```

---

## Running

```bash
cargo run --release
```

Or run the compiled binary directly:

- **macOS/Linux:** `./target/release/kalkulacka_prumeru`
- **Windows:** `target\release\kalkulacka_prumeru.exe`

---

## Usage

### Subject list

The main screen shows all your subjects with their current final grade and total number of grades. Click any subject to open it.

Use the buttons in the top bar to:
- **＋ New Subject** — create a new subject
- **📈 Overview** — see your overall GPA across all subjects
- **⬇ Export CSV** — export everything to `grades/grades_export.csv`

### Adding a subject and grades

Click **＋ New Subject**, type a name and press Enter or click Create. Once inside a subject, select a category (A, B, or C), type a grade, and press **Enter** or click **Add**. Grades are sorted automatically from best to worst.

### Removing a grade

Click the **✕** on any grade chip and confirm to remove it.

### Grade predictor

At the bottom of each subject screen, enter a target final grade and pick a category. The predictor tells you the exact next grade you need in that category to reach your target, or explains why it's not achievable and shows the best possible grade you could reach instead.

### Overview / GPA

The overview screen shows your overall GPA (average of all subject final grades), a ranked list of all subjects sorted best to worst, and a colored bar and +/− indicator showing how each subject compares to your GPA.

---

## Data storage

Grades are saved in a `grades/` folder next to the binary, one file per subject:

```
grades/
  math.json
  physics.json
  history.json
  grades_export.csv   ← created when you export
```

The JSON files are plain text and can be opened in any text editor.

---

## Grade categories

| Category | Weight |
|----------|--------|
| A        | 60%    |
| B        | 30%    |
| C        | 10%    |

If a category has no grades it is excluded and the final grade is recalculated from the remaining active categories only.
