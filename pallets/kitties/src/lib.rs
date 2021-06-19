#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure,
    traits::Randomness, RuntimeDebug, StorageDoubleMap, StorageValue,
};
use frame_system::ensure_signed;
use sp_io::hashing::blake2_128;

#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
pub struct Kitty(pub [u8; 16]);

#[derive(Encode, Decode, Clone, Copy, RuntimeDebug, PartialEq, Eq)]
enum KittyGender {
    M,
    F,
}
impl Kitty {
    fn gender(&self) -> KittyGender {
        if self.0[0] % 2 == 0 {
            KittyGender::F
        } else {
            KittyGender::M
        }
    }
}

type KittyId = u32;
pub trait Config: frame_system::Config {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
}

decl_storage! {
    trait Store for Module<T: Config> as Kitties {
        /// Stores all the kitties, key is the kitty id
        pub Kitties get(fn kitties): double_map hasher(blake2_128_concat) T::AccountId, hasher(blake2_128_concat) KittyId => Option<Kitty>;
        /// Stores the next kitty ID
        pub NextKittyId get(fn next_kitty_id): KittyId;
    }
}

decl_event! {
    pub enum Event<T> where
        <T as frame_system::Config>::AccountId,
    {
        /// A kitty is created. \[owner, kitty_id, kitty\]
        KittyCreated(AccountId, u32, Kitty),
    }
}

decl_error! {
    pub enum Error for Module<T: Config> {
        KittiesIdOverflow,
        SameGenderParents,
        MaxKittiesReachedNow,
        KittyNotExixting,
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        /// Create a new kitty
        #[weight = 1000]
        pub fn create(origin) {
            let sender = ensure_signed(origin)?;


            // TODO: ensure kitty id does not overflow
            ensure!(!(NextKittyId::get() as u32 == u32::MAX), Error::<T>::KittiesIdOverflow);
            // return Err(Error::<T>::KittiesIdOverflow.into());

            // Generate a random 128bit value
            let payload = (
                <pallet_randomness_collective_flip::Module<T> as Randomness<T::Hash>>::random_seed(),
                &sender,
                <frame_system::Module<T>>::extrinsic_index(),
            );
            let dna = payload.using_encoded(blake2_128);

            // Create and store kitty
            let kitty = Kitty(dna);
            let kitty_id = Self::next_kitty_id();
            Kitties::<T>::insert(&sender, kitty_id, kitty.clone());
            NextKittyId::put(kitty_id + 1);

            // Emit event
            Self::deposit_event(RawEvent::KittyCreated(sender, kitty_id, kitty))
        }

        #[weight = 89_000]
        pub fn breed(origin, kitti_id_1: u32, kitty_id_2: u32 ) -> DispatchResult {
            let user = ensure_signed(origin)?;
            let kitty_1 = Kitties::<T>::get(&user, kitti_id_1 as KittyId).ok_or(Error::<T>::KittyNotExixting)?;
            let kitty_2 = Kitties::<T>::get(&user, kitty_id_2 as KittyId).ok_or(Error::<T>::KittyNotExixting)?;
            let (kitty_created, new_kitty_id) = Self::breed_new(user.clone(), &kitty_1, &kitty_2)?;
            Self::deposit_event(RawEvent::KittyCreated(user, new_kitty_id, kitty_created));
            Ok(())
        }
    }
}

impl<T: Config> Module<T> {
    // add code here

    fn breed_new(
        owner: T::AccountId,
        kitty_1: &Kitty,
        kitty_2: &Kitty,
    ) -> Result<(Kitty, KittyId), &'static str> {
        // ensure first that both parents are not same sex

        ensure!(
            kitty_1.gender() != kitty_2.gender(),
            Error::<T>::SameGenderParents
        );

        let selector = 10u8;
        //now we derive gender from dna
        let mut new_dna = [0u8; 16];
        let new_dna = {
            for i in 0..kitty_1.0.len() {
                new_dna[i] = combine_dna(kitty_1.0[i], kitty_2.0[i], selector);
            }
            new_dna
        };
        let new_kitty = Kitty(new_dna);
        let mut next_kitty_id = Self::next_kitty_id();

        Kitties::<T>::insert(owner, next_kitty_id, new_kitty.clone());

        ensure!(
            next_kitty_id.checked_add(1).ok_or("err").is_ok(),
            Error::<T>::MaxKittiesReachedNow
        );
        Ok((new_kitty, next_kitty_id - 1))
    }
}

fn combine_dna(parent_1: u8, parent_2: u8, selector: u8) -> u8 {
    (!selector & parent_1) | (selector & parent_2)
}
