use glam::Vec2;

#[derive(Debug, Clone)]
pub struct ShipState {
    pub position: Vec2,
    pub velocity: Vec2,
    pub shields: f32,
    pub max_shields: f32,
    pub armor: f32,
    pub max_armor: f32,
}

impl ShipState {
    pub fn new(max_shields: f32, max_armor: f32) -> Self {
        Self {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            shields: max_shields,
            max_shields,
            armor: max_armor,
            max_armor,
        }
    }

    pub fn apply_damage(&mut self, amount: f32) -> bool {
        let absorbed = self.shields.min(amount);
        self.shields -= absorbed;
        let spill = amount - absorbed;
        if spill > 0.0 {
            self.armor = (self.armor - spill).max(0.0);
        }
        self.shields <= 0.0 && self.armor <= 0.0
    }

    pub fn regen_shields(&mut self, rate: f32, dt: f32) {
        self.shields = (self.shields + rate * dt).min(self.max_shields);
    }

    pub fn advance(&mut self, dt: f32) {
        self.position += self.velocity * dt;
    }

    pub fn is_destroyed(&self) -> bool {
        self.shields <= 0.0 && self.armor <= 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damage_is_absorbed_by_shields_first() {
        let mut ship = ShipState::new(40.0, 25.0);
        let destroyed = ship.apply_damage(10.0);
        assert!(!destroyed);
        assert_eq!(ship.shields, 30.0);
        assert_eq!(ship.armor, 25.0);
    }

    #[test]
    fn damage_spills_to_armor() {
        let mut ship = ShipState::new(40.0, 25.0);
        ship.apply_damage(50.0);
        assert_eq!(ship.shields, 0.0);
        assert_eq!(ship.armor, 15.0);
    }

    #[test]
    fn destruction_requires_both_pools_drained() {
        let mut ship = ShipState::new(40.0, 25.0);
        assert!(ship.apply_damage(65.0));
        assert!(ship.is_destroyed());
    }

    #[test]
    fn overkill_damage_clamps_at_zero() {
        let mut ship = ShipState::new(10.0, 10.0);
        ship.apply_damage(1000.0);
        assert_eq!(ship.shields, 0.0);
        assert_eq!(ship.armor, 0.0);
    }

    #[test]
    fn shield_regen_caps_at_max() {
        let mut ship = ShipState::new(40.0, 25.0);
        ship.shields = 38.0;
        ship.regen_shields(5.0, 1.0);
        assert_eq!(ship.shields, 40.0);
    }

    #[test]
    fn advance_moves_position_by_velocity() {
        let mut ship = ShipState::new(10.0, 10.0);
        ship.velocity = Vec2::new(10.0, -5.0);
        ship.advance(2.0);
        assert_eq!(ship.position, Vec2::new(20.0, -10.0));
    }
}
