//! Follow/unfollow providers and feed filtering.
//!
//! Store follows: (user, provider) -> bool
//! Store follower count per provider for leaderboard/stats.
//! Gas: O(1) follow/unfollow, O(n) get_followed_providers where n = followed count.

use soroban_sdk::{contracttype, Address, Env, Vec};

use crate::errors::SocialError;
use crate::events;

#[contracttype]
#[derive(Clone)]
pub enum SocialDataKey {
    /// (user, provider) -> true if user follows provider
    Follow(Address, Address),
    /// user -> Vec<Address> of providers they follow
    UserFollowedList(Address),
    /// provider -> u32 follower count
    FollowerCount(Address),
}

/// Check if user follows provider
pub fn is_following(env: &Env, user: &Address, provider: &Address) -> bool {
    env.storage()
        .instance()
        .get(&SocialDataKey::Follow(user.clone(), provider.clone()))
        .unwrap_or(false)
}

/// Get list of providers user follows
pub fn get_followed_providers(env: &Env, user: &Address) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&SocialDataKey::UserFollowedList(user.clone()))
        .unwrap_or_else(|| Vec::new(env))
}

/// Get follower count for provider
pub fn get_follower_count(env: &Env, provider: &Address) -> u32 {
    env.storage()
        .instance()
        .get(&SocialDataKey::FollowerCount(provider.clone()))
        .unwrap_or(0)
}

/// User follows provider. Idempotent if already following.
pub fn follow_provider(env: &Env, user: Address, provider: Address) -> Result<(), SocialError> {
    user.require_auth();

    if user == provider {
        return Err(SocialError::CannotFollowSelf);
    }

    if is_following(env, &user, &provider) {
        return Ok(()); // idempotent
    }

    // Update user's followed list
    let mut list = get_followed_providers(env, &user);
    list.push_back(provider.clone());
    env.storage()
        .instance()
        .set(&SocialDataKey::UserFollowedList(user.clone()), &list);

    // Mark follow
    env.storage().instance().set(
        &SocialDataKey::Follow(user.clone(), provider.clone()),
        &true,
    );

    // Increment follower count
    let count = get_follower_count(env, &provider) + 1;
    env.storage()
        .instance()
        .set(&SocialDataKey::FollowerCount(provider.clone()), &count);

    events::emit_follow_gained(env, user, provider.clone(), count);

    Ok(())
}

/// User unfollows provider. No error if not following.
pub fn unfollow_provider(env: &Env, user: Address, provider: Address) -> Result<(), SocialError> {
    user.require_auth();

    if !is_following(env, &user, &provider) {
        return Ok(()); // no error, idempotent
    }

    // Remove from user's followed list
    let list = get_followed_providers(env, &user);
    let mut new_list = Vec::new(env);
    for i in 0..list.len() {
        let p = list.get(i).unwrap();
        if p != provider {
            new_list.push_back(p);
        }
    }
    env.storage()
        .instance()
        .set(&SocialDataKey::UserFollowedList(user.clone()), &new_list);

    // Remove follow marker
    env.storage()
        .instance()
        .remove(&SocialDataKey::Follow(user.clone(), provider.clone()));

    // Decrement follower count
    let count = get_follower_count(env, &provider).saturating_sub(1);
    if count == 0 {
        env.storage()
            .instance()
            .remove(&SocialDataKey::FollowerCount(provider.clone()));
    } else {
        env.storage()
            .instance()
            .set(&SocialDataKey::FollowerCount(provider.clone()), &count);
    }

    events::emit_follow_lost(env, user, provider.clone(), count);

    Ok(())
}
