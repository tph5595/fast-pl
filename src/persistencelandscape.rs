use crate::birthdeath::BirthDeath;
use float_ord::FloatOrd;
use geo::{
    line_intersection::line_intersection, line_intersection::LineIntersection, Coord, Line,
};
use std::cmp::min;
use std::collections::{BinaryHeap, VecDeque};

#[derive(Debug, Clone, Copy)]
struct PersistenceMountain {
    position: Option<usize>,
    slope_rising: bool,
    birth: PointOrd,
    middle: PointOrd,
    death: PointOrd,
    id: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PointOrd {
    pub x: FloatOrd<f32>,
    pub y: FloatOrd<f32>,
}

impl Ord for PointOrd {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
         self.x.cmp(&other.x)
    }
}

impl PartialOrd for PointOrd {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, PartialEq)]
enum Direction {
    Above,
    Below,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum EventType {
    Intersection,
    Death,
    Middle,
    Birth,
}

#[derive(Debug, Clone, Copy)]
struct Event {
    value: PointOrd,
    event_type: EventType,
    parent_mountain_id: usize,
    parent_mountain2_id: Option<usize>,
}

// NOTE: This is opposite on purpose to flip to built in BinaryHeap
impl Ord for Event {
    // Compare points then event_type
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.value == other.value{
            return self.event_type.cmp(&other.event_type).reverse();
        }
        self.value.cmp(&other.value).reverse()
    }
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.parent_mountain_id == other.parent_mountain_id
            && self.parent_mountain2_id == other.parent_mountain2_id
    }
}

impl Eq for Event {}

fn create_mountain(birth: f32, death: f32, index: usize) -> PersistenceMountain {
    let half_dist = (death - birth) / 2.0;

    PersistenceMountain {
        position: None,
        slope_rising: true,
        birth: PointOrd {
            x: FloatOrd(birth),
            y: FloatOrd(0.0),
        },
        middle: PointOrd {
            x: FloatOrd(half_dist + birth),
            y: FloatOrd(half_dist),
        },
        death: PointOrd {
            x: FloatOrd(death),
            y: FloatOrd(0.0),
        },
        id: index,
    }
}

fn generate_mountains(bd_pairs: Vec<BirthDeath>) -> Vec<PersistenceMountain> {
    bd_pairs
        .into_iter()
        .filter(|BirthDeath { birth, death }| death.is_finite() && birth.is_finite())
        .enumerate()
        .map(|(i, BirthDeath { birth, death })| create_mountain(birth, death, i))
        .collect::<Vec<_>>()
}

fn generate_initial_events(mountains: Vec<PersistenceMountain>) -> Vec<Event> {
    mountains
        .into_iter()
        .flat_map(
            |PersistenceMountain {
                 birth,
                 middle,
                 death,
                 id,
                 ..
             }| {
                vec![
                    Event {
                        value: birth,
                        event_type: EventType::Birth,
                        parent_mountain_id: id,
                        parent_mountain2_id: None,
                    },
                    Event {
                        value: middle,
                        event_type: EventType::Middle,
                        parent_mountain_id: id,
                        parent_mountain2_id: None,
                    },
                    Event {
                        value: death,
                        event_type: EventType::Death,
                        parent_mountain_id: id,
                        parent_mountain2_id: None,
                    },
                ]
            },
        )
        .collect()
}

fn current_segment_start(mountain: PersistenceMountain) -> (f32, f32) {
    match mountain.slope_rising {
        true => (mountain.birth.x.0, mountain.birth.y.0),
        false => (mountain.middle.x.0, mountain.middle.y.0),
    }
}

fn current_segment_end(mountain: PersistenceMountain) -> (f32, f32) {
    match mountain.slope_rising {
        true => (mountain.middle.x.0, mountain.middle.y.0),
        false => (mountain.death.x.0, mountain.death.y.0),
    }
}

fn create_line_segment(mountain: PersistenceMountain) -> Line<f32> {
    Line {
        start: current_segment_start(mountain).into(),
        end: current_segment_end(mountain).into(),
    }
}

fn intersects_with_neighbor(m1: PersistenceMountain, m2: PersistenceMountain) -> Option<PointOrd> {
    if m1.slope_rising == m2.slope_rising {
        return None;
    }
    let inter = line_intersection(create_line_segment(m1), create_line_segment(m2));
    match inter {
        Some(LineIntersection::SinglePoint {
            intersection: Coord { x, y },
            ..
        }) => Some(PointOrd {
            x: min(FloatOrd(x), min(m1.death.x, m2.death.x)),
            y: FloatOrd(y),
        }),
        // Ignore all colinnear, not proper and no intersection results these will be resolved on
        // slope change or do not matter
        _ => None,
    }
}

fn log_to_landscape(
    mountain: PersistenceMountain,
    value: PointOrd,
    landscapes: &mut [Vec<PointOrd>],
    k: usize,
) {
    let position = mountain.position.expect("Mountain with event is dead");
    if position < k {
        landscapes[position].push(value);
    }
}

fn handle_intersection(
    status: &mut VecDeque<usize>,
    m1: PersistenceMountain,
    mountains: &mut [PersistenceMountain],
    direction_to_check: Direction,
) -> Option<Event> {
    let position = m1.position.expect("Intersection check for dead mountain");
    // Stop underflow of unsigned number
    if position == 0 && direction_to_check == Direction::Above {
        return None;
    }
    let neighbor_index = match direction_to_check {
        Direction::Below => position + 1,
        Direction::Above => position - 1,
    };

    if let Some(neighbor) = status.get(neighbor_index) {
        if let Some(intersection) = intersects_with_neighbor(m1, mountains[*neighbor]) {
            return Some(Event {
                value: intersection,
                event_type: EventType::Intersection,
                parent_mountain_id: m1.id,
                parent_mountain2_id: Some(*neighbor),
            });
        }
    }
    None
}

pub fn empty_landscape(k: usize) -> Vec<Vec<PointOrd>>{
    let mut landscapes = Vec::with_capacity(k);
    (0..k).for_each(|_| {
        let arr = Vec::new();
        landscapes.push(arr);
    });
    landscapes
}

pub fn generate(bd_pairs: Vec<BirthDeath>, k: usize, debug: bool) -> Vec<Vec<PointOrd>> {
    let landscapes = &mut empty_landscape(k);
    let mountains = &mut generate_mountains(bd_pairs);
    let events_base = &mut BinaryHeap::from(generate_initial_events(mountains.to_vec()));
    let events_int = &mut BinaryHeap::from(vec![]);
    let status = &mut VecDeque::new();

    while !events_base.is_empty() || !events_int.is_empty() {
        // Opposite on purpose due to cmp from bin heap
        let event = if events_base.peek() < events_int.peek(){
            events_int.pop().unwrap()
        }else{
            events_base.pop().unwrap()
        };
        if debug {
            println!("{:?}", event);
        }
        match event.event_type {
            EventType::Birth => {
                // Add to status structure
                let start_len = status.len();
                status.push_back(event.parent_mountain_id);
                assert!(start_len + 1 == status.len());
                let position = status.len() - 1;
                mountains[event.parent_mountain_id].position = Some(position);
                // Add to output if needed
                log_to_landscape(
                    mountains[event.parent_mountain_id],
                    event.value,
                    landscapes,
                    k,
                );
                // Check for intersections
                if let Some(new_event) = handle_intersection(
                    status,
                    mountains[event.parent_mountain_id],
                    mountains,
                    Direction::Above,
                ) {
                    events_int.push(new_event);
                }
            }
            EventType::Middle => {
                // Update status structures
                mountains[event.parent_mountain_id].slope_rising = false;
                // Add to ouput if needed
                log_to_landscape(
                    mountains[event.parent_mountain_id],
                    event.value,
                    landscapes,
                    k,
                );
                // Check for intersections
                if let Some(new_event) = handle_intersection(
                    status,
                    mountains[event.parent_mountain_id],
                    mountains,
                    Direction::Below,
                ) {
                    events_int.push(new_event);
                }
            }
            EventType::Death => {
                let pos = mountains[event.parent_mountain_id]
                    .position
                    .expect("Death of dead mountain");
                // Check for floating point mess up on death/intersection Ordering
                let weird_q = &mut VecDeque::new();
                if pos != status.len() - 1 {
                    while pos < status.len() - 1 {
                        weird_q.push_back(status.pop_back().unwrap());
                    }
                }
                // Add to ouput if needed
                log_to_landscape(
                    mountains[event.parent_mountain_id],
                    event.value,
                    landscapes,
                    k,
                );
                // remove and disable
                status.pop_back();
                mountains[event.parent_mountain_id].position = None;
                while !weird_q.is_empty() {
                    let element = weird_q.pop_back().unwrap();
                    mountains[element].position = Some(mountains[element].position.unwrap() - 1);
                    log_to_landscape(
                        mountains[element],
                        event.value,
                        landscapes,
                        k,
                    );
                    status.push_back(element);
                }
            }
            EventType::Intersection => {
                let parent_mountain2_id = event
                    .parent_mountain2_id
                    .expect("Intersection event with no second mountain");
                // Add to ouput if needed
                log_to_landscape(
                    mountains[event.parent_mountain_id],
                    event.value,
                    landscapes,
                    k,
                );
                log_to_landscape(mountains[parent_mountain2_id], event.value, landscapes, k);
                let (lower, upper) = match mountains[event.parent_mountain_id].slope_rising {
                    true => (
                        mountains[event.parent_mountain_id],
                        mountains[parent_mountain2_id],
                    ),
                    false => (
                        mountains[parent_mountain2_id],
                        mountains[event.parent_mountain_id],
                    ),
                };
                // Swap
                status.swap(
                    upper.position.expect("Dead mountain in intersection event"),
                    lower.position.expect("Dead mountain in intersection event"),
                );
                (mountains[lower.id].position, mountains[upper.id].position) =
                    (upper.position, lower.position);
                // Check for intersections
                if let Some(new_event) =
                    handle_intersection(status, mountains[lower.id], mountains, Direction::Above)
                {
                    events_int.push(new_event);
                }
                if let Some(new_event) =
                    handle_intersection(status, mountains[upper.id], mountains, Direction::Below)
                {
                    events_int.push(new_event);
                }
            }
        }
        if debug {
            println!("{:?}", status);
            println!("================================================================");
        }
    }

    landscapes.to_vec()
}
