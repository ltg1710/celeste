use std::collections::VecDeque;

use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::{
    common::{
        AnimationBundle, AnimationIndices, AnimationTimer, SPRITE_DUST_ORDER, SPRITE_HAIR_ORDER,
        SPRITE_PLAYER_ORDER, TILE_SIZE,
    },
    level::{Facing, Player, PlayerBundle, Trap, LEVEL_TRANSLATION_OFFSET},
};

// 角色头发
#[derive(Debug, Component, Clone, Copy, Default)]
pub struct Hair;

// 灰尘
#[derive(Debug, Component, Clone, Copy, Default)]
pub struct Dust;

// 玩家死亡
pub fn player_die(
    mut commands: Commands,
    mut collision_er: EventReader<CollisionEvent>,
    q_trap: Query<Entity, With<Trap>>,
) {
    for event in collision_er.iter() {
        match event {
            CollisionEvent::Started(entity1, entity2, _flags) => {
                if q_trap.contains(*entity1) {
                    info!("Player died");
                    commands.entity(*entity2).despawn_recursive();
                } else if q_trap.contains(*entity2) {
                    info!("Player died");
                    commands.entity(*entity1).despawn_recursive();
                }
            }
            _ => {}
        }
    }
}

// 玩家复活
pub fn player_revive(
    mut commands: Commands,
    q_player: Query<(), With<Player>>,
    entity_query: Query<(&Transform, &EntityInstance)>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    asset_server: Res<AssetServer>,
) {
    if q_player.is_empty() {
        for (transform, entity_instance) in &entity_query {
            if entity_instance.identifier == *"Player" {
                spawn_player(
                    &mut commands,
                    &mut texture_atlases,
                    &asset_server,
                    (transform.translation + LEVEL_TRANSLATION_OFFSET).truncate(),
                );
                break;
            }
        }
    }
}

pub fn spawn_player(
    commands: &mut Commands,
    texture_atlases: &mut ResMut<Assets<TextureAtlas>>,
    asset_server: &Res<AssetServer>,
    player_pos: Vec2,
) {
    let texture_handle = asset_server.load("textures/atlas.png");
    let texture_atlas =
        TextureAtlas::from_grid(texture_handle, Vec2::new(8.0, 8.0), 16, 11, None, None);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);

    commands.spawn(PlayerBundle {
        player: Player,
        sprite_bundle: SpriteSheetBundle {
            sprite: TextureAtlasSprite::new(1),
            texture_atlas: texture_atlas_handle,
            transform: Transform::from_translation(player_pos.extend(SPRITE_PLAYER_ORDER)),
            ..default()
        },
        animation_bundle: AnimationBundle {
            timer: AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
            indices: AnimationIndices {
                index: 0,
                sprite_indices: vec![1, 2, 3, 4],
            },
        },
        facing: Facing::Right,
        collider: Collider::ball(TILE_SIZE / 2.0),
        rigid_body: RigidBody::Dynamic,
        rotation_constraints: LockedAxes::ROTATION_LOCKED,
        velocity: Velocity::zero(),
        gravity_scale: GravityScale(10.0),
    });
}

// 角色奔跑
pub fn player_run(
    keyboard_input: Res<Input<KeyCode>>,
    mut q_player: Query<(&mut Velocity, &mut Facing), With<Player>>,
) {
    for (mut velocity, mut facing) in &mut q_player {
        if keyboard_input.pressed(KeyCode::A) {
            velocity.linvel.x = -50.0;
            *facing = Facing::Left;
        } else if keyboard_input.pressed(KeyCode::D) {
            velocity.linvel.x = 50.0;
            *facing = Facing::Right;
        } else {
            // 不按键时停止左右移动
            velocity.linvel.x = 0.0;
        }
    }
}

// 角色跳跃
pub fn player_jump(
    keyboard_input: Res<Input<KeyCode>>,
    mut q_player: Query<&mut Velocity, With<Player>>,
) {
    for mut velocity in &mut q_player {
        // 没有y轴速度，防止二段跳
        if keyboard_input.pressed(KeyCode::K) && velocity.linvel.y.abs() < 0.1 {
            velocity.linvel = Vec2::new(0.0, 300.0);
        }
    }
}

// 角色冲刺/冲撞
pub fn player_dash(
    mut commands: Commands,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    asset_server: Res<AssetServer>,
    keyboard_input: Res<Input<KeyCode>>,
    mut q_player: Query<(&mut Velocity, &Facing, &Transform), With<Player>>,
    mut spawn_dust_cd: Local<f32>,
    time: Res<Time>,
) {
    for (mut velocity, facing, transform) in &mut q_player {
        if keyboard_input.pressed(KeyCode::J) {
            if *facing == Facing::Left {
                velocity.linvel = Vec2::new(-200.0, 0.0);
            }
            if *facing == Facing::Right {
                velocity.linvel = Vec2::new(200.0, 0.0);
            }

            if *spawn_dust_cd > 0.0 {
                *spawn_dust_cd -= time.delta_seconds();
            } else {
                spawn_dust(
                    &mut commands,
                    &mut texture_atlases,
                    &asset_server,
                    transform.translation.truncate(),
                );
                // 重置cd
                *spawn_dust_cd = 0.02;
            }
        }
    }
}

// 地面奔跑动画
pub fn animate_run(
    keyboard_input: Res<Input<KeyCode>>,
    mut q_player: Query<
        (
            &Facing,
            &mut AnimationTimer,
            &mut AnimationIndices,
            &mut TextureAtlasSprite,
        ),
        With<Player>,
    >,
    time: Res<Time>,
) {
    // TODO 暂时通过这个判断处于run状态
    if keyboard_input.any_pressed([KeyCode::A, KeyCode::D]) {
        for (facing, mut timer, mut indices, mut sprite) in &mut q_player {
            timer.0.tick(time.delta());
            if timer.0.just_finished() {
                // 切换到下一个sprite
                sprite.index = if indices.index == indices.sprite_indices.len() - 1 {
                    indices.index = 0;
                    indices.sprite_indices[indices.index]
                } else {
                    indices.index += 1;
                    indices.sprite_indices[indices.index]
                };
                if *facing == Facing::Left {
                    sprite.flip_x = true;
                } else {
                    sprite.flip_x = false;
                }
            }
        }
    }
}

// 跳跃动画
pub fn animate_jump(
    mut q_player: Query<(&Velocity, &Facing, &mut TextureAtlasSprite), With<Player>>,
) {
    for (velocity, facing, mut sprite) in &mut q_player {
        // TODO 暂时通过这个判断处于jump状态
        if velocity.linvel.y.abs() > 0.1 {
            sprite.index = 3;
            if *facing == Facing::Left {
                sprite.flip_x = true;
            } else {
                sprite.flip_x = false;
            }
        }
    }
}

// 站立动画
pub fn animate_stand(
    mut q_player: Query<(&Velocity, &Facing, &mut TextureAtlasSprite), With<Player>>,
) {
    for (velocity, facing, mut sprite) in &mut q_player {
        // TODO 暂时通过这个判断处于stand状态
        if velocity.linvel.x.abs() < 0.1 && velocity.linvel.y.abs() < 0.1 {
            sprite.index = 1;
            if *facing == Facing::Left {
                sprite.flip_x = true;
            } else {
                sprite.flip_x = false;
            }
        }
    }
}

// 创建角色头发
pub fn spawn_hair(
    mut commands: Commands,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    asset_server: Res<AssetServer>,
    q_hair: Query<(), With<Hair>>,
    q_player: Query<&Transform, With<Player>>,
) {
    if !q_player.is_empty() && q_hair.is_empty() {
        let columns = 6;
        let texture_handle = asset_server.load("textures/hair.png");
        let texture_atlas = TextureAtlas::from_grid(
            texture_handle,
            Vec2::new(8.0, 8.0),
            columns,
            1,
            Some(Vec2::new(1.0, 1.0)),
            Some(Vec2::new(1.0, 1.0)),
        );
        let texture_atlas_handle = texture_atlases.add(texture_atlas);

        for player_transfrom in &q_player {
            for i in 0..columns {
                commands.spawn((
                    Hair,
                    SpriteSheetBundle {
                        sprite: TextureAtlasSprite {
                            index: i,
                            color: Color::RED,
                            ..default()
                        },
                        texture_atlas: texture_atlas_handle.clone(),
                        transform: Transform::from_translation(
                            player_transfrom
                                .translation
                                .truncate()
                                .extend(SPRITE_HAIR_ORDER),
                        ),
                        ..default()
                    },
                ));
            }
        }
    }
}

pub fn despawn_hair(
    mut commands: Commands,
    q_hair: Query<Entity, With<Hair>>,
    q_player: Query<&Transform, With<Player>>,
) {
    if q_player.is_empty() && !q_hair.is_empty() {
        for entity in &q_hair {
            commands.entity(entity).despawn();
        }
    }
}

pub fn animate_hair(
    mut q_hair: Query<(&mut Transform, &mut TextureAtlasSprite), (With<Hair>, Without<Player>)>,
    q_player: Query<(&Transform, &Facing), With<Player>>,
    mut hair_flow: Local<VecDeque<Vec2>>,
) {
    if q_player.is_empty() || q_hair.is_empty() {
        return;
    }
    // hair_flow记录最近5帧player位置
    hair_flow.push_front(q_player.single().0.translation.truncate());
    if hair_flow.len() > 5 {
        hair_flow.pop_back();
    }

    // 头发排序
    let mut hair: Vec<(Mut<Transform>, Mut<TextureAtlasSprite>)> = q_hair.iter_mut().collect();
    hair.sort_by(|single_hair1, single_hair2| single_hair1.1.index.cmp(&single_hair2.1.index));

    // 依次往每个bucket（hair_flow槽）里放，直至放满，多余hair放到最后的bucket
    let bucket_size = hair.iter().len() / hair_flow.len();
    let mut bucket_index = 0;
    let mut count = 0;
    for single_hair in hair.iter_mut() {
        single_hair.0.translation = hair_flow
            .get(bucket_index)
            .unwrap()
            .extend(SPRITE_HAIR_ORDER);
        single_hair.1.flip_x = if *q_player.single().1 == Facing::Left {
            true
        } else {
            false
        };
        count += 1;
        if count > bucket_size {
            // 下一个bucket
            bucket_index = if bucket_index < hair_flow.len() - 1 {
                bucket_index + 1
            } else {
                bucket_index
            };
            count = 0;
        }
    }
}

pub fn spawn_dust(
    commands: &mut Commands,
    texture_atlases: &mut ResMut<Assets<TextureAtlas>>,
    asset_server: &Res<AssetServer>,
    dust_pos: Vec2,
) {
    let texture_handle = asset_server.load("textures/atlas.png");
    let texture_atlas =
        TextureAtlas::from_grid(texture_handle, Vec2::new(8.0, 8.0), 16, 11, None, None);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);

    commands.spawn((
        Dust,
        SpriteSheetBundle {
            sprite: TextureAtlasSprite::new(29),
            texture_atlas: texture_atlas_handle,
            transform: Transform::from_translation(dust_pos.extend(SPRITE_DUST_ORDER)),
            ..default()
        },
        AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
        AnimationIndices {
            index: 0,
            sprite_indices: vec![29, 30, 31],
        },
    ));
}

pub fn animate_dust(
    mut commands: Commands,
    mut q_dust: Query<
        (
            Entity,
            &mut AnimationTimer,
            &mut AnimationIndices,
            &mut TextureAtlasSprite,
        ),
        With<Dust>,
    >,
    time: Res<Time>,
) {
    for (entity, mut timer, mut indices, mut sprite) in &mut q_dust {
        timer.0.tick(time.delta());
        if timer.0.just_finished() {
            // 切换到下一个sprite
            if indices.index == indices.sprite_indices.len() - 1 {
                commands.entity(entity).despawn();
            } else {
                indices.index += 1;
                sprite.index = indices.sprite_indices[indices.index];
            };
        }
    }
}
