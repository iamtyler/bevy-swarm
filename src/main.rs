use bevy::prelude::*;
use rand;


const PLAYER_SPEED: f32       = 100.0;
const PLAYER_BODY_RADIUS: f32 = 18.0;

const MONSTER_SPEED: f32       = 50.0;
const MONSTER_BODY_RADIUS: f32 = 10.0;
const MONSTER_BODY_MASS: f32   = 10.0;

const MONSTER_SPAWN_DISTANCE: f32       = 300.0;
const MONSTER_SPAWN_LIMIT: u32          = 300;
const MONSTER_SPAWN_PERIOD_SECONDS: f32 = 0.6;

const BLAST_RADIUS: f32               = 50.0;
const BLAST_LIFETIME_SECONDS: f32     = 0.3;
const BLAST_SPAWN_PERIOD_SECONDS: f32 = 3.0;

const COLLISION_DISPLACEMENT_FACTOR: f32 = 0.2;

#[derive(Default)]
struct MonsterStats {
    spawned: u32,
    killed: u32,
}

impl MonsterStats {
    fn clear(&mut self) {
        self.spawned = 0;
        self.killed = 0;
    }

    fn count(&self) -> u32 {
        if self.spawned > self.killed {
            self.spawned - self.killed
        }
        else {
            0
        }
    }
}

struct MonsterSpawnTimer(Timer);

impl MonsterSpawnTimer {
    fn new() -> MonsterSpawnTimer {
        let mut timer = Timer::from_seconds(MONSTER_SPAWN_PERIOD_SECONDS, true);
        timer.pause();

        MonsterSpawnTimer(timer)
    }
}

struct BlastSpawnTimer(Timer);

impl BlastSpawnTimer {
    fn new() -> BlastSpawnTimer {
        let mut timer = Timer::from_seconds(BLAST_SPAWN_PERIOD_SECONDS, true);
        timer.pause();

        BlastSpawnTimer(timer)
    }
}

#[derive(Component)]
struct Blast {
    lifetime: Timer,
    circle: Circle,
}

impl Blast {
    fn new() -> Blast {
        Blast{
            lifetime: Timer::from_seconds(BLAST_LIFETIME_SECONDS, false),
            circle: Circle::new(BLAST_RADIUS),
        }
    }
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Monster;

#[derive(Component, Default, PartialEq)]
struct Position {
    current: Vec2,
    change: Vec2,
}

struct NewGameEvent;

impl Position {
    fn new(current: Vec2) -> Position {
        Position{
            current,
            change: Vec2::ZERO,
        }
    }

    fn apply(&mut self, change: Vec2) {
        self.current += change;
        self.change = change;
    }

    fn apply_add(&mut self, change: Vec2) {
        self.current += change;
        self.change += change;
    }
}

#[derive(Component, Default)]
struct Velocity {
    direction: Vec2,
    speed: f32,
}

impl Velocity {
    fn new(direction: Vec2, speed: f32) -> Velocity {
        Velocity{
            direction,
            speed,
        }
    }

    fn is_zero(&self) -> bool {
        self.direction == Vec2::ZERO || self.speed == 0.0
    }

    fn change_for_seconds(&self, seconds: f32) -> Vec2 {
        if self.is_zero() {
            Vec2::ZERO
        }
        else {
            self.direction * (self.speed * seconds)
        }
    }
}

#[derive(Component)]
struct Body {
    circle: Circle,
    mass: Option<f32>,
    collision: Collision,
}

impl Body {
    fn new(circle: Circle, mass: Option<f32>) -> Body {
        Body{
            circle,
            mass,
            collision: Collision{
                displacement: Vec2::ZERO,
                is_firm: false,
            },
        }
    }
}

struct Collision {
    displacement: Vec2,
    is_firm: bool,
}

impl Collision {
    fn clear(&mut self) {
        self.displacement = Vec2::ZERO;
        self.is_firm = false;
    }
}

struct Circle {
    radius: f32,
}

impl Circle {
    fn new(radius: f32) -> Circle {
        Circle {
            radius,
        }
    }
}

#[derive(SystemLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub enum Movement {
    Input,
    Player,
    Monster,
    Damage,
    Spread,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(
            player_direction
                .label(Movement::Input)
                .before(Movement::Player),
        )
        .add_system(
            movement
                .label(Movement::Player),
        )
        .add_system(
            monster_direction
                .label(Movement::Monster)
                .after(Movement::Player),
        )
        .add_system(
            damage_collision
                .label(Movement::Damage)
                .after(Movement::Monster),
        )
        .add_system(
            spread_collision
                .label(Movement::Spread)
                .after(Movement::Damage),
        )
        .add_system(
            blast_collision
                .after(Movement::Spread),
        )
        .add_system(spawn_monster)
        .add_system(spawn_blast)
        .add_system(blast_lifetime)
        .add_system(new_game)
        .insert_resource(MonsterStats::default())
        .insert_resource(MonsterSpawnTimer::new())
        .insert_resource(BlastSpawnTimer::new())
        .add_system_set_to_stage(
            CoreStage::PostUpdate,
            SystemSet::new()
                .with_system(position_translation),
        )
        .add_event::<NewGameEvent>()
        .run();
}

fn setup(
    mut commands: Commands,
    mut new_game_writer: EventWriter<NewGameEvent>,
) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    new_game_writer.send(NewGameEvent);
}

fn new_game(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut new_game_reader: EventReader<NewGameEvent>,
    players: Query<Entity, With<Player>>,
    monsters: Query<Entity, With<Monster>>,
    blasts: Query<Entity, With<Blast>>,
    mut monster_stats: ResMut<MonsterStats>,
    mut monster_spawn_timer: ResMut<MonsterSpawnTimer>,
    mut blast_spawn_timer: ResMut<BlastSpawnTimer>,
) {
    // Only fire if event was sent
    if !new_game_reader.iter().next().is_some() {
        return;
    }

    // Clear state
    for player in players.iter() {
        commands.entity(player).despawn();
    }
    for monster in monsters.iter() {
        commands.entity(monster).despawn();
    }
    for blast in blasts.iter() {
        commands.entity(blast).despawn();
    }
    monster_stats.clear();

    // Create player
    commands
        .spawn_bundle(SpriteBundle {
            texture: asset_server.load("player.png"),
            transform: Transform {
                scale: Vec3::new(4.0, 4.0, 1.0),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Player)
        .insert(Position::new(Vec2::ZERO))
        .insert(Velocity::new(Vec2::ZERO, PLAYER_SPEED))
        .insert(Body::new(Circle::new(PLAYER_BODY_RADIUS), None));

    // Reset and unpause spawn timers
    monster_spawn_timer.0.reset();
    monster_spawn_timer.0.unpause();
    blast_spawn_timer.0.reset();
    blast_spawn_timer.0.unpause();
}

fn random_unit() -> Vec2 {
    let x = rand::random::<f32>() * 2.0 - 1.0;
    let y = rand::random::<f32>() * 2.0 - 1.0;

    Vec2::new(x, y).normalize_or_zero()
}

fn spawn_blast(
    time: Res<Time>,
    mut spawn_timer: ResMut<BlastSpawnTimer>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    player: Query<&Position, With<Player>>,
) {
    spawn_timer.0.tick(time.delta());
    if !spawn_timer.0.just_finished() {
        return;
    }

    let target = if let Some(p) = player.iter().next() {
        p.current
    }
    else {
        return
    };

    commands
        .spawn_bundle(SpriteBundle {
            texture: asset_server.load("blast.png"),
            sprite: Sprite{
                custom_size: Some(Vec2::splat(BLAST_RADIUS * 2.0)),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Blast::new())
        .insert(Position::new(target));
}

fn blast_lifetime(
    time: Res<Time>,
    mut commands: Commands,
    mut blasts: Query<(&mut Blast, &mut Sprite, Entity)>,
) {
    for mut blast in blasts.iter_mut() {
        blast.0.lifetime.tick(time.delta());
        if blast.0.lifetime.just_finished() {
            commands.entity(blast.2).despawn();
            continue;
        }
    }
}

fn spawn_monster(
    time: Res<Time>,
    mut spawn_timer: ResMut<MonsterSpawnTimer>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    player: Query<&Position, With<Player>>,
    mut monster_stats: ResMut<MonsterStats>,
) {
    spawn_timer.0.tick(time.delta());
    if !spawn_timer.0.just_finished() {
        return;
    }

    if monster_stats.count() >= MONSTER_SPAWN_LIMIT {
        return;
    }

    let target = if let Some(p) = player.iter().next() {
        p.current
    }
    else {
        return
    };

    let direction = random_unit();
    let position = target + (direction * MONSTER_SPAWN_DISTANCE);

    commands
        .spawn_bundle(SpriteBundle {
            texture: asset_server.load("monster.png"),
            transform: Transform {
                scale: Vec3::new(2.0, 2.0, 1.0),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Monster)
        .insert(Position::new(position))
        .insert(Velocity::new(Vec2::ZERO, MONSTER_SPEED))
        .insert(Body::new(Circle::new(MONSTER_BODY_RADIUS), Some(MONSTER_BODY_MASS)));

    monster_stats.spawned += 1;
}

fn movement(
    time: Res<Time>,
    mut query: Query<(&Velocity, &mut Position)>,
) {
    for (v, mut p) in query.iter_mut() {
        if v.is_zero() {
            continue;
        }

        p.apply(v.change_for_seconds(time.delta_seconds()));
    }
}

fn position_translation(
    player: Query<&Position, With<Player>>,
    mut query: Query<(&Position, &mut Transform)>,
) {
    let offset = if let Some(p) = player.iter().next() {
        p.current
    }
    else {
        return
    };

    for (p, mut t) in query.iter_mut() {
        t.translation.x = p.current.x - offset.x;
        t.translation.y = p.current.y - offset.y;
    }
}

fn monster_direction(
    player: Query<&Position, With<Player>>,
    mut monsters: Query<(&Position, &mut Velocity), With<Monster>>,
) {
    let target = if let Some(p) = player.iter().next() {
        p.current
    }
    else {
        return
    };

    for (p, mut v) in monsters.iter_mut() {
        let difference = target - p.current;
        v.direction = difference.normalize_or_zero();
    }
}

fn player_direction(
    keyboard_input: Res<Input<KeyCode>>,
    mut velocities: Query<&mut Velocity, With<Player>>,
) {
    // Pull one player velocity out of the query
    if let Some(mut v) = velocities.iter_mut().next() {
        // Start with no direction
        let mut direction = Vec2::ZERO;

        // Read horizontal direction, preferring right
        if keyboard_input.pressed(KeyCode::Right) {
            direction.x = 1.0;
        }
        else if keyboard_input.pressed(KeyCode::Left) {
            direction.x = -1.0;
        }

        // Read vertical direction, preferring up
        if keyboard_input.pressed(KeyCode::Up) {
            direction.y = 1.0;
        }
        else if keyboard_input.pressed(KeyCode::Down) {
            direction.y = -1.0;
        }

        // Set normalized (or zero) direction
        v.direction = direction.normalize_or_zero();
    }
}

fn damage_collision(
    players: Query<(&Body, &Position), With<Player>>,
    monsters: Query<(&Body, &Position), With<Monster>>,
    mut new_game_writer: EventWriter<NewGameEvent>,
) {
    for player in players.iter() {
        for monster in monsters.iter() {
            let (did_collide, _) = collide_circles(
                (&player.0.circle, player.1.current),
                (&monster.0.circle, monster.1.current),
            );

            if did_collide {
                new_game_writer.send(NewGameEvent);
                return;
            }
        }
    }
}

fn blast_collision(
    mut commands: Commands,
    blasts: Query<(&Blast, &Position)>,
    monsters: Query<(&Body, &Position, Entity), With<Monster>>,
    mut monster_stats: ResMut<MonsterStats>,
) {
    for blast in blasts.iter() {
        for monster in monsters.iter() {
            let (did_collide, _) = collide_circles(
                (&blast.0.circle, blast.1.current),
                (&monster.0.circle, monster.1.current),
            );

            if did_collide {
                commands.entity(monster.2).despawn();
                monster_stats.killed += 1;
            }
        }
    }
}

fn spread_collision(
    mut bodies: Query<(&mut Body, &mut Position)>,
) {
    // Detect collisions and accumulate displacements
    let mut combinations = bodies.iter_combinations_mut();
    while let Some([mut a, mut b]) = combinations.fetch_next() {
        // Detect overlap
        let (did_collide, overlap) = collide_circles(
            (&a.0.circle, a.1.current),
            (&b.0.circle, b.1.current),
        );

        // No work if no collision
        if !did_collide {
            continue;
        }

        // Handle case where both bodies are immovable
        if a.0.mass.is_none() && b.0.mass.is_none() {
            // Do nothing I guess
            continue;
        }

        // Handle immovable a
        if a.0.mass.is_none() || a.0.collision.is_firm {
            b.0.collision.displacement = -overlap;
            b.0.collision.is_firm = true;
            continue;
        }

        // Handle immovable b
        if b.0.mass.is_none() || b.0.collision.is_firm {
            a.0.collision.displacement = overlap;
            a.0.collision.is_firm = true;
            continue;
        }

        // Move each according to mass
        let a_mass = a.0.mass.unwrap();
        let b_mass = b.0.mass.unwrap();
        let total_mass = a_mass + b_mass;

        let a_factor = b_mass / total_mass;
        let b_factor = a_mass / total_mass;

        a.0.collision.displacement += overlap * a_factor;
        b.0.collision.displacement -= overlap * b_factor;
    }

    // Apply displacements
    for mut body in bodies.iter_mut() {
        if body.0.collision.displacement != Vec2::ZERO {
            body.1.apply_add(body.0.collision.displacement * COLLISION_DISPLACEMENT_FACTOR);
        }

        body.0.collision.clear();
    }
}

fn collide_circles(
    a: (&Circle, Vec2),
    b: (&Circle, Vec2),
) -> (bool, Vec2) {
    // Determine overlap threshold from radii
    let radius_sum = a.0.radius + b.0.radius;
    let radius_sum_squared = radius_sum * radius_sum;

    // Determine position difference
    let difference = a.1 - b.1;

    // Determine distance from difference
    let distance_squared = difference.length_squared();

    // Determine overlap
    let overlap = radius_sum_squared - distance_squared;

    // Generate overlap vector
    if overlap <= 0.0 {
        (false, Vec2::ZERO)
    }
    else if distance_squared == 0.0 {
        (true, random_unit() * overlap.sqrt())
    } else {
        (true, difference.normalize_or_zero() * overlap.sqrt())
    }
}
