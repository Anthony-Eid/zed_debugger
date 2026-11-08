//! A debugger torture test.
//!
//! Each section exercises something debuggers (and the DAP adapters and UIs on
//! top of them) commonly get wrong: unusual data layouts, huge or cyclic data,
//! deep stacks, many threads stopping at once, async state machines, caught
//! panics, and hostile program output.
//!
//! Search for `BREAK:` comments for suggested breakpoint locations and what to
//! inspect there. Everything is deterministic, so values are stable across runs.
//!
//! ```sh
//! cargo run --bin torture             # run every section
//! cargo run --bin torture -- threads  # run one section
//! cargo run --bin torture -- --list   # list sections
//! ```

use std::{
    any::Any,
    borrow::Cow,
    cell::{Cell, RefCell},
    collections::{hash_map::DefaultHasher, BTreeMap, HashMap, HashSet, VecDeque},
    convert::Infallible,
    ffi::{CString, OsString},
    fmt::Debug,
    hash::BuildHasherDefault,
    hint::black_box,
    io::{self, Write},
    marker::PhantomData,
    num::NonZeroU32,
    panic::{self, AssertUnwindSafe},
    path::PathBuf,
    rc::{Rc, Weak},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Barrier, Mutex, OnceLock, RwLock,
    },
    thread,
    time::Duration,
};

/// A named chunk of the torture test that can be run on its own.
type Section = (&'static str, fn());

/// Every section, in the order they run. The name is the CLI filter argument.
const SECTIONS: &[Section] = &[
    ("primitives", primitives),
    ("strings", strings),
    ("collections", collections),
    ("enums", enums),
    ("pointers", smart_pointers),
    ("recursive", recursive_data),
    ("traits", traits_and_generics),
    ("scopes", shadowing),
    ("stack", deep_stack),
    ("threads", threads),
    ("async", async_state_machines),
    ("panics", panics),
    ("output", noisy_output),
];

/// Globals show up in a different place than locals in most debuggers.
static GLOBAL_COUNTER: AtomicUsize = AtomicUsize::new(0);
static GLOBAL_NAME: &str = "torture-test";
static GLOBAL_CONFIG: OnceLock<BTreeMap<&'static str, u32>> = OnceLock::new();

thread_local! {
    /// Each thread sees a different value: check the debugger reads the right one.
    static THREAD_LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn main() {
    // An optional section name (or `--list`) as the first argument.
    let filter = std::env::args().nth(1);
    if filter.as_deref() == Some("--list") {
        for (name, _) in SECTIONS {
            println!("{name}");
        }
        return;
    }

    // Initialize lazily so the global starts out empty and only fills in here:
    // a breakpoint before this line should show the OnceLock as uninitialized.
    GLOBAL_CONFIG.get_or_init(|| BTreeMap::from([("depth", 5_000), ("workers", 4)]));
    let mut ran = 0;
    for (name, section) in SECTIONS {
        // No filter means run everything.
        if filter.as_deref().is_some_and(|wanted| wanted != *name) {
            continue;
        }
        println!("== {name} ==");
        section();
        GLOBAL_COUNTER.fetch_add(1, Ordering::Relaxed);
        ran += 1;
    }

    // Nothing matched: probably a typo in the section name.
    if ran == 0 {
        eprintln!("unknown section {filter:?}; try --list");
        std::process::exit(2);
    }
    println!("{GLOBAL_NAME}: ran {ran} section(s)");
}

// ---------------------------------------------------------------------------
// Primitives and layout oddities
// ---------------------------------------------------------------------------

/// The same four bytes read as an integer or a float. Debuggers can't know
/// which field is active, so they should show both.
#[repr(C)]
union IntOrFloat {
    int: u32,
    float: f32,
}

/// No padding: `value` starts at offset 1, so it is misaligned. Debuggers
/// that assume natural alignment read garbage here.
#[repr(C, packed)]
struct Packed {
    tag: u8,
    value: u64,
}

/// Over-aligned: 64 bytes in size even though it holds a single `u32`.
#[repr(align(64))]
struct CacheLine {
    hot: u32,
}

/// A zero-sized unit struct: it has no address worth reading.
struct Zst;
/// Also zero-sized, but declared with braces, which some tools treat differently.
struct Empty {}
/// A newtype: ideally shown as `Meters(1.5)`, not as a struct with field `__0`.
struct Meters(f64);

fn primitives() {
    // Extremes of each integer width, to catch sign and width mistakes.
    let small: (i8, u8, i16, u16) = (i8::MIN, u8::MAX, -1, 0xBEEF);
    // 128-bit integers don't fit in a register, and many tools truncate them.
    let wide: (i128, u128) = (i128::MIN, u128::MAX);
    let floats = [
        0.0f64,
        -0.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MIN_POSITIVE / 2.0, // subnormal
        f64::EPSILON,
    ];
    let third: f32 = 1.0 / 3.0; // should display as an f32, not widened to f64

    // ASCII, Latin-1, NUL, a crab (outside the BMP) and an invisible zero-width space.
    let chars = ['a', 'é', '\0', '\u{1F980}', '\u{200B}'];
    let unit = ();
    let zst = Zst;
    let empty = Empty {};
    let phantom: PhantomData<Vec<String>> = PhantomData;
    let meters = Meters(1.5);
    let union = IntOrFloat { float: 1.0 };
    let bits = unsafe { union.int }; // 0x3f800000, the bit pattern of 1.0f32
    let packed = Packed {
        tag: 7,
        value: 0x0123_4567_89AB_CDEF,
    };
    let packed_value = packed.value; // copy out: references to packed fields are UB
    let aligned = CacheLine { hot: 42 };
    let aligned_addr = &aligned as *const CacheLine as usize;
    let empty_array: [u8; 0] = [];
    // A 2D array filled with 0..16, so each cell's value gives away its position.
    let mut grid = [[0u8; 4]; 4];
    for (row, cells) in grid.iter_mut().enumerate() {
        for (col, cell) in cells.iter_mut().enumerate() {
            *cell = (row * 4 + col) as u8;
        }
    }
    let null: *const u8 = std::ptr::null();
    // Non-null and aligned, but it points at nothing.
    let dangling: *const u64 = std::ptr::NonNull::dangling().as_ptr();
    let fn_pointer: fn() = primitives;

    // BREAK: Check -0.0 vs 0.0, NaN, subnormals, u128::MAX, the ZST and empty
    // array (size 0), the union shown both ways, the packed struct's misaligned
    // field, a 64-byte aligned address, null/dangling raw pointers (expanding
    // them must not crash), and a function pointer with its symbol name.
    // `black_box` keeps the optimizer from deleting values that are otherwise
    // unused, so they stay inspectable even in release builds.
    black_box((&small, &wide, &floats, &third, &chars, &unit, &zst, &empty));
    black_box((&phantom, &meters, &union, &packed, &aligned, &empty_array));
    black_box((&grid, &null, &dangling, &fn_pointer));
    println!(
        "bits={bits:#x} packed={packed_value:#x} tag={} aligned={} meters={} hot={}",
        packed.tag,
        aligned_addr % 64 == 0,
        meters.0,
        aligned.hot
    );
}

// ---------------------------------------------------------------------------
// Strings
// ---------------------------------------------------------------------------

fn strings() {
    let ascii = String::from("hello, debugger");
    let unicode = "naïve café — 日本語 — 🦀🦀";
    let combining = "e\u{301}"; // looks like one char, is two code points
    let right_to_left = "שלום עולם"; // bidi text can scramble neighboring columns
    let escapes = "tab\tnewline\nquote\"backslash\\nul\0bell\x07";
    // Raw terminal escape codes: these should show as text, not be interpreted.
    let ansi = "\x1b[31mred\x1b[0m \x1b[1mbold\x1b[0m";
    let empty = String::new();
    let reserved = String::with_capacity(1024); // len 0, capacity 1024
    let long_line = "x".repeat(10_000); // one line with no break points
    let huge = "0123456789abcdef".repeat(65_536); // 1 MiB: watch for UI stalls
    let substring = &unicode[..6]; // "naïve": 5 chars, 6 bytes
    let not_utf8: &[u8] = b"\xff\xfe not utf-8 \x00 end";
    let lossy = String::from_utf8_lossy(not_utf8); // a Cow::Owned holding U+FFFD
    let os = os_strings();
    // Has a space and non-ASCII chars, both of which trip up naive quoting.
    let path = PathBuf::from("/tmp/zed debugger/ünïcödé/file.rs");
    let borrowed: Cow<str> = Cow::Borrowed("borrowed");
    let owned: Cow<str> = Cow::Owned("owned".to_uppercase());
    // Fat pointers to str: (pointer, length) pairs that should still show as text.
    let boxed: Box<str> = "boxed str".into();
    let shared: Rc<str> = Rc::from("shared str");
    // NUL-terminated, unlike every other string here.
    let c_string = CString::new("c string").expect("no interior nul");

    // BREAK: Strings should show as text, not byte arrays. Check that multibyte
    // and combining chars aren't split, escapes are escaped, ANSI codes don't
    // color the variables pane, the 1 MiB string is truncated rather than
    // freezing the UI, and the non-UTF-8 bytes are displayed sensibly.
    black_box((
        &ascii,
        &unicode,
        &combining,
        &right_to_left,
        &escapes,
        &ansi,
    ));
    black_box((&empty, &reserved, &substring, &not_utf8, &lossy, &os, &path));
    black_box((&borrowed, &owned, &boxed, &shared, &c_string));
    println!(
        "lengths: long={} huge={} substring={substring:?} lossy={lossy:?}",
        long_line.len(),
        huge.len()
    );
}

/// A valid OS string, plus one that is not valid UTF-8 (only possible on Unix,
/// where OS strings are arbitrary bytes).
#[cfg(unix)]
fn os_strings() -> (OsString, OsString) {
    use std::os::unix::ffi::OsStringExt;
    (
        OsString::from("os string"),
        OsString::from_vec(vec![b'b', b'a', b'd', 0xff, 0xfe]),
    )
}

/// Fallback for platforms where OS strings can't hold arbitrary bytes.
#[cfg(not(unix))]
fn os_strings() -> (OsString, OsString) {
    (
        OsString::from("os string"),
        OsString::from("other os string"),
    )
}

// ---------------------------------------------------------------------------
// Collections
// ---------------------------------------------------------------------------

/// A fixed-seed hasher so HashMap iteration order is the same on every run.
type StableHashMap<K, V> = HashMap<K, V, BuildHasherDefault<DefaultHasher>>;

fn collections() {
    let empty: Vec<u32> = Vec::new();
    let single = vec![1];
    let squares: Vec<u64> = (0..100_000).map(|i| i * i).collect();
    // Jagged: the inner vectors have lengths 0 through 4.
    let nested: Vec<Vec<i32>> = (0..5).map(|n| (0..n).collect()).collect();

    // Force the ring buffer to wrap: many pretty-printers show the wrong order.
    let mut wrapped: VecDeque<i32> = VecDeque::with_capacity(8);
    wrapped.extend(0..6);
    wrapped.drain(..4);
    wrapped.extend(6..12); // now the logical start is in the middle of the buffer

    // Each word maps to its byte values.
    let mut map: StableHashMap<String, Vec<u32>> = StableHashMap::default();
    for word in ["alpha", "beta", "gamma", "delta", "epsilon"] {
        map.insert(word.to_string(), word.bytes().map(u32::from).collect());
    }
    // Composite keys, sorted by the tuple, plus a NaN value for good measure.
    let tuple_keys: BTreeMap<(i32, &str), Option<f64>> = BTreeMap::from([
        ((1, "one"), Some(1.0)),
        ((-1, "minus one"), None),
        ((0, ""), Some(f64::NAN)),
    ]);
    // Duplicate letters collapse: "debugger" has only 6 distinct chars.
    let set: HashSet<char, BuildHasherDefault<DefaultHasher>> = "debugger".chars().collect();
    let boxed_slice: Box<[u16]> = vec![10, 20, 30].into_boxed_slice();
    let window = &squares[10..20]; // a slice into the middle of a big Vec
    let options = [Some(1u8), None, Some(255), None];
    // A lazy iterator adaptor chain: nothing has been evaluated yet, so the
    // debugger can only show the nested adaptor structs.
    let iterator = squares.iter().skip(5).step_by(3).take(4);

    // BREAK: Expanding `squares` (100k elements) should page lazily rather than
    // fetching everything. `wrapped` should read 4, 5, 6, ..., 11 even though its
    // storage has wrapped around. Map, set and slice views should show elements,
    // not raw buffers and capacities.
    black_box((&empty, &single, &nested, &wrapped, &map, &tuple_keys, &set));
    black_box((&boxed_slice, &window, &options, &iterator));
    let sampled: Vec<_> = iterator.collect();
    println!(
        "squares={} wrapped={wrapped:?} sampled={sampled:?}",
        squares.len()
    );
}

// ---------------------------------------------------------------------------
// Enums and niche layouts
// ---------------------------------------------------------------------------

/// One of every variant kind: unit, struct-like, tuple-like, heap data, and
/// recursive.
#[derive(Debug)]
#[allow(dead_code)] // fields exist to be inspected in the debugger
enum Shape {
    Unit,
    Circle { radius: f64 },
    Rect(f64, f64),
    Poly(Vec<(i32, i32)>),
    Nested(Box<Shape>),
}

/// Explicit, non-contiguous discriminants in a one-byte tag.
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
enum Level {
    Low = 1,
    Mid = 50,
    High = 255,
}

/// Signed discriminants: a tag read as unsigned shows -1 as 18446744073709551615.
#[repr(i64)]
#[derive(Debug, Clone, Copy)]
enum Signed {
    Negative = -1,
    Max = i64::MAX,
}

fn enums() {
    let shapes = vec![
        Shape::Unit,
        Shape::Circle { radius: 2.5 },
        Shape::Rect(3.0, 4.0),
        Shape::Poly(vec![(0, 0), (1, 0), (0, 1)]),
        Shape::Nested(Box::new(Shape::Nested(Box::new(Shape::Circle {
            radius: 0.5,
        })))),
    ];
    let levels = [Level::Low, Level::Mid, Level::High];
    let signed = [Signed::Negative, Signed::Max];
    let target = 7u32;
    // These use niche optimization: no separate tag, `None` is a forbidden value.
    let some_ref: Option<&u32> = Some(&target);
    let none_ref: Option<&u32> = None;
    let some_nonzero: Option<NonZeroU32> = NonZeroU32::new(7);
    let none_nonzero: Option<NonZeroU32> = NonZeroU32::new(0);
    let none_box: Option<Box<Shape>> = None;
    // Stacked niches: all three layers of Option fit in a single byte.
    let nested_option: Option<Option<Option<bool>>> = Some(Some(None));
    let ok: Result<u32, String> = Ok(3);
    let err: Result<u32, String> = Err("nope".into());
    let infallible: Result<u8, Infallible> = Ok(1); // the Err variant is uninhabited
    let ordering = 1.cmp(&2); // Ordering::Less, stored as -1

    // BREAK: Each enum should show its active variant and fields, not a raw tag
    // and a union of all variants. Niche-optimized Options must not show
    // `Some(0x0)`. `Level::High` is 255 and `Signed::Negative` is -1.
    black_box((&shapes, &levels, &signed, &some_ref, &none_ref));
    black_box((&some_nonzero, &none_nonzero, &none_box, &nested_option));
    black_box((&ok, &err, &infallible, &ordering));
    let discriminants: Vec<u8> = levels.iter().map(|&level| level as u8).collect();
    println!(
        "shapes={} discriminants={discriminants:?} signed={:?}",
        shapes.len(),
        signed.map(|s| s as i64)
    );
}

// ---------------------------------------------------------------------------
// Smart pointers and interior mutability
// ---------------------------------------------------------------------------

fn smart_pointers() {
    let rc = Rc::new(vec![1, 2, 3]);
    let rc_clone = Rc::clone(&rc); // strong count 2
    let live_weak = Rc::downgrade(&rc);
    // The Rc is dropped at the end of this block, so this Weak can't upgrade.
    let dead_weak: Weak<String> = {
        let temporary = Rc::new(String::from("gone"));
        Rc::downgrade(&temporary)
    };
    let never_alive: Weak<u8> = Weak::new(); // dangling sentinel pointer

    let arc = Arc::new(Mutex::new(vec!["shared"; 3])); // two layers of wrapping
    let held = Mutex::new(5);
    let guard = held.lock().expect("not poisoned"); // locked at the breakpoint
    let rwlock = RwLock::new(String::from("readers"));
    // Two readers hold the lock at the same time.
    let reader_one = rwlock.read().expect("not poisoned");
    let reader_two = rwlock.read().expect("not poisoned");
    let exclusive = RefCell::new(vec![1]);
    let borrowed_mut = exclusive.borrow_mut(); // borrow flag is -1
    let shared = RefCell::new(0);
    let (borrow_one, borrow_two) = (shared.borrow(), shared.borrow()); // flag is 2
    let cell = Cell::new(10);
    let pinned = Box::pin(String::from("pinned")); // Pin<Box<String>> should see through Pin
    let boxes = Box::new(Box::new(Box::new(42)));
    let value = 9;
    let reference_chain: &&&&i32 = &&&&value;
    let atomic = AtomicUsize::new(3); // stored in an UnsafeCell inside the atomic

    // One empty OnceLock and one filled: the debugger should tell them apart.
    let unset_once: OnceLock<String> = OnceLock::new();
    let set_once: OnceLock<String> = OnceLock::new();
    set_once
        .set(String::from("initialized"))
        .expect("first set");

    // BREAK: Check strong/weak counts (rc: 2 strong, 1 weak), that the dead and
    // never-alive Weaks don't crash when expanded, that lock and borrow state is
    // visible, and that pointer chains (`boxes`, `reference_chain`) auto-deref.
    black_box((&rc_clone, &live_weak, &dead_weak, &never_alive, &arc));
    black_box((&guard, &reader_one, &reader_two, &borrowed_mut));
    black_box((&borrow_one, &borrow_two, &cell, &pinned, &boxes));
    black_box((&reference_chain, &atomic, &unset_once, &set_once));
    println!(
        "rc strong={} weak={} dead={} boxes={}",
        Rc::strong_count(&rc),
        Rc::weak_count(&rc),
        dead_weak.upgrade().is_none(),
        ***boxes
    );
}

// ---------------------------------------------------------------------------
// Recursive and cyclic data
// ---------------------------------------------------------------------------

/// A classic cons list: each node owns the rest of the list.
enum List {
    Cons(i32, Box<List>),
    Nil,
}

/// A tree with strong links to children and weak links back to the parent, so
/// it can be walked in both directions without leaking.
struct TreeNode {
    name: String,
    parent: RefCell<Weak<TreeNode>>,
    children: RefCell<Vec<Rc<TreeNode>>>,
}

/// A node in a ring of strong references that never terminates on its own.
struct CycleNode {
    id: u32,
    next: RefCell<Option<Rc<CycleNode>>>,
}

/// Creates a node and, if a parent is given, links the two together.
fn tree_node(name: &str, parent: Option<&Rc<TreeNode>>) -> Rc<TreeNode> {
    let node = Rc::new(TreeNode {
        name: name.to_string(),
        parent: RefCell::new(parent.map(Rc::downgrade).unwrap_or_default()),
        children: RefCell::new(Vec::new()),
    });
    if let Some(parent) = parent {
        parent.children.borrow_mut().push(Rc::clone(&node));
    }
    node
}

fn recursive_data() {
    // A 1000-deep linked list: expanding it one level at a time is tedious, and
    // expanding it recursively should not hang the debugger.
    let list = (0..1_000)
        .rev()
        .fold(List::Nil, |tail, value| List::Cons(value, Box::new(tail)));

    // root -> {left, middle, right} -> two leaves each.
    let root = tree_node("root", None);
    for branch in ["left", "middle", "right"] {
        let child = tree_node(branch, Some(&root));
        for leaf in 0..2 {
            tree_node(&format!("{branch}-{leaf}"), Some(&child));
        }
    }
    // "right-1": the last leaf, two levels down.
    let leaf = Rc::clone(&root.children.borrow()[2].children.borrow()[1]);

    // A reference cycle: a -> b -> c -> a. Expanding `next` repeatedly loops forever.
    let a = Rc::new(CycleNode {
        id: 1,
        next: RefCell::new(None),
    });
    let b = Rc::new(CycleNode {
        id: 2,
        next: RefCell::new(Some(Rc::clone(&a))),
    });
    let c = Rc::new(CycleNode {
        id: 3,
        next: RefCell::new(Some(Rc::clone(&b))),
    });
    *a.next.borrow_mut() = Some(Rc::clone(&c)); // close the loop

    // BREAK: Expand `a.next` a few times: the debugger should keep up without
    // trying to walk the whole cycle. `leaf.parent` is a Weak that leads back up
    // the tree; `root.children` leads back down.
    black_box((&list, &root, &leaf, &a));
    // Walk the list iteratively; a recursive walk might overflow the stack.
    let (mut length, mut sum) = (0, 0);
    let mut cursor = &list;
    while let List::Cons(value, next) = cursor {
        length += 1;
        sum += value;
        cursor = next;
    }
    let parent_name = leaf.parent.borrow().upgrade().map(|p| p.name.clone());
    let cycle_ids = [a.id, b.id, c.id];
    println!(
        "list={length} sum={sum} leaf={} parent={parent_name:?} cycle={cycle_ids:?}",
        leaf.name
    );

    a.next.borrow_mut().take(); // break the cycle so nothing leaks
}

// ---------------------------------------------------------------------------
// Traits, generics, closures and macros
// ---------------------------------------------------------------------------

/// A trait implemented by unrelated types so we can build trait objects.
trait Describe {
    fn describe(&self) -> String;
}

impl Describe for Shape {
    fn describe(&self) -> String {
        format!("shape {self:?}")
    }
}

impl Describe for u32 {
    fn describe(&self) -> String {
        format!("number {self}")
    }
}

/// A blanket impl: one source location, many monomorphized copies.
impl<T: Debug> Describe for Vec<T> {
    fn describe(&self) -> String {
        format!("vec of {} items", self.len())
    }
}

/// BREAK: A breakpoint on the line below resolves to one location per type it
/// is instantiated with (u8, &str, Vec<char>, Shape). The debugger should
/// show all of them as a single breakpoint and stop at each.
fn identity<T: Debug>(value: T) -> T {
    black_box(&value);
    value
}

#[inline(always)]
fn always_inlined(x: i32) -> i32 {
    // BREAK: Inlined code: the frame may be missing from the call stack, or
    // shown as a synthetic inline frame. Stepping into it should still work.
    x * 2 + 1
}

/// Generates one function per `name => amount` pair.
macro_rules! make_adders {
    ($($name:ident => $amount:expr),* $(,)?) => {
        $(
            fn $name(x: i32) -> i32 {
                // BREAK: Code expanded from a macro. Stepping through it jumps
                // between the call site and this definition.
                x + $amount
            }
        )*
    };
}

make_adders!(add_one => 1, add_ten => 10);

/// Returns an opaque `impl Iterator` whose real type is an unnameable
/// closure-based adaptor.
fn counting_iterator(limit: u32) -> impl Iterator<Item = u32> {
    (0..limit).filter(|n| n % 3 == 0)
}

/// A deliberately absurd type, for testing how long type names are displayed.
type Monstrous =
    HashMap<String, Vec<Option<Result<Rc<RefCell<BTreeMap<u8, Box<[char]>>>>, String>>>>;

fn traits_and_generics() {
    let objects: Vec<Box<dyn Describe>> = vec![
        Box::new(Shape::Rect(1.0, 2.0)),
        Box::new(17u32),
        Box::new(vec!['a', 'b']),
    ];
    let dyn_ref: &dyn Describe = &5u32; // (data pointer, vtable pointer)

    // Type-erased: the debugger has to use the vtable to recover the type.
    let any_value: Box<dyn Any> = Box::new(String::from("downcast me"));

    let offset = 100;
    let mut call_count = 0;
    let borrowing = |x: i32| x + offset; // captures `offset` by reference
    let mut mutating = |x: i32| {
        call_count += 1; // captures `call_count` by mutable reference
        x * call_count
    };
    let owned_vec = vec![1, 2, 3];
    let moving = move || owned_vec.iter().sum::<i32>(); // owns `owned_vec`

    // A closure behind a trait object: its captures are hidden behind a vtable.
    let boxed_fn: Box<dyn Fn(i32) -> i32> = Box::new(move |x| x - offset);

    let monstrous: Monstrous = HashMap::from([(
        String::from("key"),
        vec![
            Some(Ok(Rc::new(RefCell::new(BTreeMap::from([(
                1,
                vec!['x', 'y'].into_boxed_slice(),
            )]))))),
            Some(Err(String::from("error"))),
            None,
        ],
    )]);

    // BREAK: `objects` and `dyn_ref` are fat pointers: the debugger should show
    // the concrete type behind each. Closures should show their captures.
    // `monstrous` has a very long type name that should not wreck the layout.
    black_box((&objects, &dyn_ref, &any_value, &boxed_fn, &monstrous));
    let descriptions: Vec<String> = objects.iter().map(|o| o.describe()).collect();
    let downcast = any_value.downcast_ref::<String>().cloned();
    let closures = (
        borrowing(1),
        mutating(2),
        mutating(3),
        moving(),
        boxed_fn(1),
    );
    // Each call is a separate instantiation of `identity`.
    let generic = (
        identity(1u8),
        identity("str"),
        identity(vec!['a']),
        identity(Shape::Unit),
    );
    // Stepping into this line visits two macro-generated functions, then inlined code.
    let inlined = always_inlined(add_ten(add_one(0)));
    let from_iterator: Vec<u32> = counting_iterator(10).collect();
    println!(
        "{descriptions:?} {} {downcast:?} {closures:?}",
        dyn_ref.describe()
    );
    println!("{generic:?} inlined={inlined} iterator={from_iterator:?}");
}

// ---------------------------------------------------------------------------
// Shadowing and scopes
// ---------------------------------------------------------------------------

fn shadowing() {
    let value = 1;
    let value = value.to_string(); // same name, now a String
    {
        let value = value.len() as f64; // same name, now an f64, inner scope
        for value in 0..2u8 {
            // BREAK: Four variables named `value` are live here with four types.
            // The variables pane should show the innermost one, or all of them
            // clearly distinguished.
            black_box(value);
        }
        black_box(value);
    }
    // Back in the outer scope: the String is visible again, now wrapped.
    let value = Some(value);
    if let Some(value) = &value {
        black_box(value);
    }
    // Match bindings shadow `value` once more, with different binding modes.
    let summary = match value {
        Some(ref inner) if inner.is_empty() => String::from("empty"),
        Some(inner) => format!("value {inner}"),
        None => String::from("none"),
    };
    println!("{summary}");
}

// ---------------------------------------------------------------------------
// Deep call stacks
// ---------------------------------------------------------------------------

/// Recurses until `level == max`, leaving one frame per level on the stack.
fn recurse(level: u32, max: u32, trail: &mut Vec<u32>) -> u32 {
    let frame_local = [level; 4]; // a distinct value in every frame
    trail.push(level);
    if level == max {
        // BREAK: About 5,000 frames deep. The call stack should load lazily
        // and selecting a frame near the bottom should show that frame's locals.
        return black_box(frame_local)[0];
    }
    // Not a tail call in practice: Rust doesn't guarantee tail-call elimination.
    recurse(level + 1, max, trail)
}

/// Mutual recursion: the call stack alternates between `is_even` and `is_odd`.
fn is_even(n: u32) -> bool {
    if n == 0 {
        true
    } else {
        is_odd(n - 1)
    }
}

/// The other half of `is_even`.
fn is_odd(n: u32) -> bool {
    if n == 0 {
        false
    } else {
        is_even(n - 1)
    }
}

fn deep_stack() {
    // Read from the global so the depth can be inspected (and changed) in one place.
    let depth = GLOBAL_CONFIG.get().map_or(5_000, |config| config["depth"]);
    // Run on a thread with a large stack so this works the same under `cargo test`.
    let (deepest, trail_len, even) = thread::Builder::new()
        .name(String::from("deep-stack"))
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            let mut trail = Vec::new();
            let deepest = recurse(0, depth, &mut trail);
            (deepest, trail.len(), is_even(501))
        })
        .expect("spawn deep-stack thread")
        .join()
        .expect("deep-stack thread panicked");
    println!("deepest={deepest} frames={trail_len} is_even(501)={even}");
}

// ---------------------------------------------------------------------------
// Threads
// ---------------------------------------------------------------------------

fn threads() {
    const WORKERS: usize = 4;
    // Named workers, one unnamed thread and the main thread all meet here.
    let barrier = Arc::new(Barrier::new(WORKERS + 2));
    let total = Arc::new(AtomicUsize::new(0));

    let mut handles: Vec<thread::JoinHandle<usize>> = (0..WORKERS)
        .map(|id| {
            let barrier = Arc::clone(&barrier);
            let total = Arc::clone(&total);
            thread::Builder::new()
                .name(format!("worker-{id}"))
                .spawn(move || {
                    // A different length per worker, so the threads are easy to tell apart.
                    let payload = vec![id; id + 1];
                    THREAD_LOG.with(|log| log.borrow_mut().push(format!("worker {id}")));
                    barrier.wait();
                    // BREAK: All four workers reach this line at nearly the same
                    // time. The debugger should report every stopped thread, let
                    // you switch between them, and show each thread's own
                    // `payload` and `THREAD_LOG`.
                    total.fetch_add(payload.len(), Ordering::SeqCst);
                    id * 10
                })
                .expect("spawn worker")
        })
        .collect();

    // A thread without a name: the debugger has to fall back to an ID.
    let unnamed_barrier = Arc::clone(&barrier);
    handles.push(thread::spawn(move || {
        unnamed_barrier.wait();
        999
    }));

    let sleeper = thread::Builder::new()
        .name(String::from("sleeper"))
        .spawn(|| thread::sleep(Duration::from_millis(200))) // blocked in a syscall
        .expect("spawn sleeper");

    barrier.wait(); // the main thread is the last participant
    let results: Vec<usize> = handles
        .into_iter()
        .map(|handle| handle.join().expect("thread panicked"))
        .collect();
    sleeper.join().expect("sleeper panicked");

    // Scoped threads borrow from this stack frame instead of owning their data.
    let data: Vec<u32> = (1..=100).collect();
    let (left, right) = data.split_at(data.len() / 2);
    let (left_sum, right_sum) = thread::scope(|scope| {
        let left = scope.spawn(|| left.iter().sum::<u32>());
        let right = scope.spawn(|| right.iter().sum::<u32>());
        (left.join().expect("left"), right.join().expect("right"))
    });

    println!(
        "results={results:?} total={} sums=({left_sum}, {right_sum})",
        total.load(Ordering::SeqCst)
    );
}

// ---------------------------------------------------------------------------
// Async
// ---------------------------------------------------------------------------

/// The innermost future: yields once so it has a real suspension point.
async fn leaf(n: u32) -> u32 {
    let before = n * 2;
    smol::future::yield_now().await;
    // BREAK: `before` survived a suspension point, so it lives inside the
    // future's state machine rather than on the stack. The call stack shows
    // executor internals instead of a logical async call chain.
    before + 1
}

/// Awaits two `leaf` futures in sequence, nesting their state machines inside
/// its own.
async fn middle(n: u32) -> u32 {
    let first = leaf(n).await;
    let second = leaf(n + 1).await;
    first + second
}

fn async_state_machines() {
    // An unpolled future: inspecting it shows a compiler-generated state machine.
    let pending = middle(99);
    black_box(&pending);
    drop(pending);

    let (joined, spawned) = smol::block_on(async {
        // Two futures polled concurrently on the same thread.
        let joined = smol::future::zip(middle(1), middle(10)).await;
        // Runs on smol's global executor threads, not the current one.
        let spawned = smol::spawn(middle(100)).await;
        (joined, spawned)
    });
    println!("joined={joined:?} spawned={spawned}");
}

// ---------------------------------------------------------------------------
// Panics
// ---------------------------------------------------------------------------

fn panics() {
    // BREAK: Set a function breakpoint on `rust_panic` (or enable a Rust panic
    // exception breakpoint) to stop at each panic below, even though all of
    // them are caught.
    // Swap in a quiet hook so expected panics don't spew backtraces, and put
    // the original back afterwards.
    let previous_hook = panic::take_hook();
    panic::set_hook(Box::new(|info| eprintln!("  caught: {info}")));

    let empty: Vec<u8> = Vec::new();
    let cell = RefCell::new(0);
    let outcomes = [
        attempt("explicit", || panic!("explicit panic with payload {}", 42)),
        attempt("index", || _ = black_box(black_box(&empty)[3])),
        attempt("expect", || {
            _ = black_box(black_box(None::<u8>).expect("value missing"))
        }),
        attempt("divide", || _ = black_box(1 / black_box(0))),
        // Only panics in debug builds; release builds wrap silently.
        attempt("overflow", || _ = black_box(black_box(u8::MAX) + 1)),
        attempt("refcell", || {
            let _first = cell.borrow_mut();
            let _second = cell.borrow_mut();
        }),
        // A payload that isn't a string: the hook can't print it.
        attempt("custom payload", || {
            panic::panic_any(vec![1, 2, 3]);
        }),
    ];

    panic::set_hook(previous_hook);
    println!("panics caught: {outcomes:?}");
}

/// Runs `f`, catching any panic, and reports whether it panicked.
fn attempt(label: &'static str, f: impl FnOnce()) -> (&'static str, bool) {
    let panicked = panic::catch_unwind(AssertUnwindSafe(f)).is_err();
    (label, panicked)
}

// ---------------------------------------------------------------------------
// Hostile output
// ---------------------------------------------------------------------------

/// Writes output that debug consoles commonly mangle.
fn noisy_output() {
    let stdout = io::stdout();
    let stderr = io::stderr();

    // Interleaved streams: the console should keep them in order.
    for i in 0..20 {
        if i % 2 == 0 {
            writeln!(stdout.lock(), "stdout line {i}").ok();
        } else {
            writeln!(stderr.lock(), "stderr line {i}").ok();
        }
    }

    // Hold the lock for the rest of the section so the writes aren't interleaved.
    let mut out = stdout.lock();
    writeln!(
        out,
        "\x1b[31mred\x1b[0m \x1b[32mgreen\x1b[0m \x1b[1;4mbold underline\x1b[0m"
    )
    .ok();
    writeln!(out, "{}", "long line ".repeat(1_000)).ok(); // 10,000 chars on one line
    for percent in (0..=100).step_by(25) {
        write!(out, "\rprogress {percent:>3}%").ok(); // carriage returns, no newline
    }
    writeln!(out).ok();
    // Raw bytes that aren't valid UTF-8; the console must not drop the line.
    out.write_all(b"invalid utf-8: \xff\xfe\xfd\n").ok();
    writeln!(out, "emoji and wide chars: 🦀🦀 日本語 | tab\there").ok();
    write!(out, "no trailing newline").ok();
    out.flush().ok();
    drop(out); // release the lock before `println!` takes it again
    println!();
}

#[cfg(test)]
mod tests {
    use super::SECTIONS;

    /// Smoke test: every section should run to completion without panicking.
    #[test]
    fn every_section_runs() {
        for (name, section) in SECTIONS {
            println!("== {name} ==");
            section();
        }
    }
}
