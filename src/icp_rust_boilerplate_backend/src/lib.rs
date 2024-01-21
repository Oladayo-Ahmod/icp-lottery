#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};
use ic_cdk::api::time;
use rand_chacha::rand_core::{SeedableRng, CryptoRngCore};
use ic_cdk::{trap, caller};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct LotteryTicket {
    id: u64,
    owner: String,
    numbers: Vec<u32>,
    created_at: u64,
    updated_at: Option<u64>,
    lottery_draw_num: u64
}

// Implement Storable and BoundedStorable traits for LotteryTicket
impl Storable for LotteryTicket {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for LotteryTicket {
    const MAX_SIZE: u32 = 1024;  // Set an appropriate max size for your struct
    const IS_FIXED_SIZE: bool = false;
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct LotteryDraw {
    id: u64,
    winning_numbers: Vec<u32>,
    draw_time: u64,
    participants: Vec<String>,
    over: bool
}

// Implement Storable and BoundedStorable traits for LotteryDraw
impl Storable for LotteryDraw {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for LotteryDraw {
    const MAX_SIZE: u32 = 1024;  // Set an appropriate max size for your struct
    const IS_FIXED_SIZE: bool = false;
}

#[derive(candid::CandidType, Deserialize, Serialize)]
enum LotteryError {
    NotFound { msg: String },
    InvalidNumbers { msg: String },
    LotteryDrawOver
}

thread_local! {
    static LOTTERY_MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static LOTTERY_TICKET_ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(LOTTERY_MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );

    static LOTTERY_DRAW_ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(LOTTERY_MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1))), 0)
            .expect("Cannot create a counter")
    );

    static LOTTERY_TICKET_STORAGE: RefCell<StableBTreeMap<u64, LotteryTicket, Memory>> = RefCell::new(
        StableBTreeMap::init(LOTTERY_MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2))))
    );

    static LOTTERY_DRAW_STORAGE: RefCell<StableBTreeMap<u64, LotteryDraw, Memory>> = RefCell::new(
        StableBTreeMap::init(LOTTERY_MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(3))))
    );
}

// Function to buy a lottery ticket
#[ic_cdk::update]
fn buy_lottery_ticket(numbers: Vec<u32>, draw_id: u64) -> Result<LotteryTicket, LotteryError> {
    // Validate the numbers
    if numbers.len() != 6 || numbers.iter().any(|&num| num > 49 || num == 0) {
        return Err(LotteryError::InvalidNumbers { msg: "Invalid lottery numbers, there should be 6 numbers between 1 and 49".to_string() });
    }

    let mut draw = LOTTERY_DRAW_STORAGE.with(|service| {
        service
            .borrow_mut()
            .get(&draw_id)
            .ok_or_else(|| LotteryError::NotFound {
                msg: format!("Lottery draw with id={} not found", draw_id),
            })
    })?;

    if draw.over {
        return Err(LotteryError::LotteryDrawOver)
    }

    let id = LOTTERY_TICKET_ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("cannot increment id counter");
    let caller_str = caller().to_string();

    if !draw.participants.contains(&caller_str){
        draw.participants.push(caller_str.clone()); 
    };

    let ticket = LotteryTicket {
        id,
        owner: caller_str,
        numbers,
        created_at: time(),
        updated_at: None,
        lottery_draw_num: draw.id
    };
    LOTTERY_DRAW_STORAGE.with(|m| m.borrow_mut().insert(draw.id, draw.clone()));
    LOTTERY_TICKET_STORAGE.with(|m| m.borrow_mut().insert(id, ticket.clone()));
    Ok(ticket)
}

// Function to check lottery ticket by ID
#[ic_cdk::query]
fn check_lottery_ticket(id: u64) -> Result<LotteryTicket, LotteryError> {
    LOTTERY_TICKET_STORAGE.with(|service| {
        service
            .borrow_mut()
            .get(&id)
            .ok_or(LotteryError::NotFound {
                msg: format!("Lottery ticket with id={} not found", id),
            })
    })
}

// Function to conduct a lottery draw
#[ic_cdk::update]
fn create_lottery_draw() -> Result<LotteryDraw, LotteryError> {
    let id = LOTTERY_DRAW_ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("cannot increment id counter");

    let draw = LotteryDraw {
        id,
        winning_numbers: Vec::new(),
        draw_time: 0,
        participants: Vec::new(),
        over: false
    };

    LOTTERY_DRAW_STORAGE.with(|m| m.borrow_mut().insert(id, draw.clone()));
    Ok(draw)
}

// Function to conduct a lottery draw
#[ic_cdk::update]
async fn conduct_lottery_draw(draw_id: u64) -> Result<LotteryDraw, LotteryError> {
    let mut draw = LOTTERY_DRAW_STORAGE.with(|service| {
        service
            .borrow_mut()
            .get(&draw_id)
            .ok_or_else(|| LotteryError::NotFound {
                msg: format!("Lottery draw with id={} not found", draw_id),
            })
    })?;

    if draw.over {
        return Err(LotteryError::LotteryDrawOver)
    }

    let winning_numbers = generate_winning_numbers().await.expect("Failed to conduct lottery draw");
    draw.over = true;
    draw.winning_numbers = winning_numbers;
    draw.draw_time = time();
    LOTTERY_DRAW_STORAGE.with(|m| m.borrow_mut().insert(draw_id, draw.clone()));
    Ok(draw)
}


// Function to get all lottery tickets
#[ic_cdk::query]
fn get_all_lottery_tickets() -> Result<Vec<LotteryTicket>, LotteryError> {
    let tickets = LOTTERY_TICKET_STORAGE.with(|m| m.borrow().iter().map(|(_, v)| v.clone()).collect::<Vec<_>>());
    if tickets.len() == 0 {
        return Err(LotteryError::NotFound { msg: "No lottery tickets found".to_string() });
    }
    Ok(tickets)
}

// Function to get all lottery draws
#[ic_cdk::query]
fn get_all_lottery_draws() -> Result<Vec<LotteryDraw>, LotteryError> {
    let draws = LOTTERY_DRAW_STORAGE.with(|m| m.borrow().iter().map(|(_, v)| v.clone()).collect::<Vec<_>>());
    if draws.len() == 0 {
        return Err(LotteryError::NotFound { msg: "No lottery draws found".to_string() });
    }
    Ok(draws)
}


pub async fn generate_winning_numbers() -> Result<Vec<u32>, String> {
    let mut i = 0;
    let mut winning_numbers = Vec::new();
    while i < 6{
        let rnd_buffer: (Vec<u8>,) = match ic_cdk::api::management_canister::main::raw_rand().await {
            Ok(result) => result,
            Err(err) => {
                ic_cdk::println!("Error invoking raw_rand: {:?} {}", err.0, err.1);
                return Err(err.1);
            }
        };
        let seed = rnd_buffer.0[..].try_into().unwrap_or_else(|_| {
            trap(&format!(
                    "when creating seed from raw_rand output, expected raw randomness to be of length 32, got {}",
                    rnd_buffer.0.len()
                    ));
        });
        let mut rand = rand_chacha::ChaCha20Rng::from_seed(seed);
        let random_number = rand.as_rngcore().next_u32() % 50;
        i += 1;
        winning_numbers.push(random_number);
    }
    Ok(winning_numbers)
}

// Export the candid interface
ic_cdk::export_candid!();
