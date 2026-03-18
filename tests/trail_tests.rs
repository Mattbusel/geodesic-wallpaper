//! Integration tests for `TrailBuffer`.

use geodesic_wallpaper::trail::TrailBuffer;

fn red() -> [f32; 4] {
    [1.0, 0.0, 0.0, 1.0]
}

#[test]
fn new_buffer_empty() {
    let buf = TrailBuffer::new(10, red());
    assert_eq!(buf.count, 0);
    assert_eq!(buf.ordered_vertices().len(), 0);
}

#[test]
fn push_increments_len() {
    let mut buf = TrailBuffer::new(10, red());
    buf.push([1.0, 2.0, 3.0]);
    assert_eq!(buf.count, 1);
    buf.push([4.0, 5.0, 6.0]);
    assert_eq!(buf.count, 2);
}

#[test]
fn push_returns_correct_len_via_ordered() {
    let mut buf = TrailBuffer::new(5, red());
    for i in 0..5 {
        buf.push([i as f32, 0.0, 0.0]);
    }
    assert_eq!(buf.ordered_vertices().len(), 5);
}

#[test]
fn buffer_wraps_at_capacity() {
    let cap = 4;
    let mut buf = TrailBuffer::new(cap, red());
    for i in 0..8u32 {
        buf.push([i as f32, 0.0, 0.0]);
    }
    assert_eq!(buf.count, cap);
    assert_eq!(buf.ordered_vertices().len(), cap);
}

#[test]
fn points_returned_in_order_before_wrap() {
    let mut buf = TrailBuffer::new(5, red());
    for i in 0..5u32 {
        buf.push([i as f32, 0.0, 0.0]);
    }
    let verts = buf.ordered_vertices();
    let positions: Vec<f32> = verts.iter().map(|v| v.position[0]).collect();
    assert_eq!(positions, vec![0.0, 1.0, 2.0, 3.0, 4.0]);
}

#[test]
fn points_returned_in_order_after_wrap() {
    let cap = 4;
    let mut buf = TrailBuffer::new(cap, red());
    for i in 0..6u32 {
        buf.push([i as f32, 0.0, 0.0]);
    }
    // The last 4 inserted are [2, 3, 4, 5].
    let verts = buf.ordered_vertices();
    let positions: Vec<f32> = verts.iter().map(|v| v.position[0]).collect();
    assert_eq!(positions, vec![2.0, 3.0, 4.0, 5.0], "order after wrap: {positions:?}");
}

#[test]
fn clear_resets_count() {
    let mut buf = TrailBuffer::new(8, red());
    for i in 0..8 {
        buf.push([i as f32, 0.0, 0.0]);
    }
    buf.clear();
    assert_eq!(buf.count, 0);
    assert_eq!(buf.ordered_vertices().len(), 0);
}

#[test]
fn zero_capacity_clamped_to_one() {
    let buf = TrailBuffer::new(0, red());
    assert_eq!(buf.capacity, 1);
}

#[test]
fn fade_alpha_increases_tail_to_head() {
    let cap = 10;
    let mut buf = TrailBuffer::new(cap, red());
    for i in 0..cap {
        buf.push([i as f32, 0.0, 0.0]);
    }
    let verts = buf.ordered_vertices();
    let first = verts[0].color[3];
    let last = verts[cap - 1].color[3];
    assert!(first < last, "alpha should increase: first={first} last={last}");
}
