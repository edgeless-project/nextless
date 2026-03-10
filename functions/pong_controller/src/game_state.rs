// SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use crate::PADDLE_LEN;

#[derive(Debug)]
pub struct GameState {
    size_y: u64,
    size_x: u64,
    paddle_1_y: u64,
    paddle_2_y: u64,
    ball_x: u64,
    ball_y: u64,
    ball_direction: BallDirection,
    points_1: u64,
    points_2: u64,
}

#[derive(Debug)]
enum BallDirection {
    Left,
    Right,
    LeftUp,
    RightUp,
    LeftDown,
    RightDown,
}

enum PaddleHit {
    None,
    Up,
    Center,
    Down,
}

impl GameState {
    pub fn new(size_y: u64, size_x: u64) -> Self {
        Self {
            size_y,
            size_x,
            paddle_1_y: size_y / 2 - super::PADDLE_LEN / 2,
            paddle_2_y: size_y / 2 - super::PADDLE_LEN / 2,
            ball_x: size_x / 2 - super::BALL_SIZE / 2,
            ball_y: size_y / 2 - super::BALL_SIZE / 2,
            ball_direction: BallDirection::Left,
            points_1: 0,
            points_2: 0,
        }
    }

    pub fn logic_update(&mut self) {
        if self.ball_x == 0 {
            self.reset_ball(BallDirection::Right);
            self.scored(2);
            return;
        }

        if self.ball_x == self.size_x - 1 - super::BALL_SIZE {
            self.reset_ball(BallDirection::Left);
            self.scored(1);
            return;
        }

        match self.ball_direction {
            BallDirection::Left => {
                self.ball_x = self.ball_x.saturating_sub(1);
            }
            BallDirection::Right => {
                self.ball_x = std::cmp::min(self.ball_x + 1, self.size_x - super::BALL_SIZE);
            }
            BallDirection::LeftUp => {
                self.ball_x = self.ball_x.saturating_sub(1);
                self.ball_y = self.ball_y.saturating_sub(1);
            }
            BallDirection::RightUp => {
                self.ball_x = std::cmp::min(self.ball_x + 1, self.size_x - super::BALL_SIZE);
                self.ball_y = self.ball_y.saturating_sub(1);
            }
            BallDirection::LeftDown => {
                self.ball_x = self.ball_x.saturating_sub(1);
                self.ball_y = std::cmp::min(self.ball_y + 1, self.size_y - super::BALL_SIZE);
            }
            BallDirection::RightDown => {
                self.ball_x = std::cmp::min(self.ball_x + 1, self.size_x - super::BALL_SIZE);
                self.ball_y = std::cmp::min(self.ball_y + 1, self.size_y - super::BALL_SIZE);
            }
        }

        if self.ball_y == 0 {
            match self.ball_direction {
                BallDirection::LeftUp => {
                    self.ball_direction = BallDirection::LeftDown;
                }
                BallDirection::RightUp => {
                    self.ball_direction = BallDirection::RightDown;
                }
                _ => {}
            }
        }

        if self.ball_y == self.size_y - super::BALL_SIZE {
            match self.ball_direction {
                BallDirection::LeftDown => {
                    self.ball_direction = BallDirection::LeftUp;
                }
                BallDirection::RightDown => {
                    self.ball_direction = BallDirection::RightUp;
                }
                _ => {}
            }
        }

        if self.ball_x == super::PADDLE_WIDTH {
            match self.paddle_hit(self.paddle_1_y) {
                PaddleHit::None => {}
                PaddleHit::Up => {
                    self.ball_direction = BallDirection::RightUp;
                }
                PaddleHit::Center => {
                    self.ball_direction = BallDirection::Right;
                }
                PaddleHit::Down => self.ball_direction = BallDirection::RightDown,
            }
        }

        if self.ball_x == self.size_x - 1 - super::PADDLE_WIDTH - super::BALL_SIZE {
            match self.paddle_hit(self.paddle_2_y) {
                PaddleHit::None => {}
                PaddleHit::Up => {
                    self.ball_direction = BallDirection::LeftUp;
                }
                PaddleHit::Center => {
                    self.ball_direction = BallDirection::Left;
                }
                PaddleHit::Down => self.ball_direction = BallDirection::LeftDown,
            }
        }
    }

    pub fn move_paddle_up(&mut self, id: usize) {
        log::trace!("Move Paddle {id} up.");
        if id == 1 {
            if self.paddle_1_y > 0 {
                self.paddle_1_y -= 1;
            }
        }

        if id == 2 {
            if self.paddle_2_y > 0 {
                self.paddle_2_y -= 1;
            }
        }
    }

    pub fn move_paddle_down(&mut self, id: usize) {
        log::trace!("Move Paddle {id} down.");
        if id == 1 {
            if (self.paddle_1_y + PADDLE_LEN - 1) < self.size_y - 1 {
                self.paddle_1_y += 1;
            }
        }

        if id == 2 {
            if (self.paddle_2_y + PADDLE_LEN - 1) < self.size_y - 1 {
                self.paddle_2_y += 1;
            }
        }
    }

    pub fn as_render_request(&self) -> super::messages::PongRenderRequest {
        super::messages::PongRenderRequest {
            paddle_1_y: self.paddle_1_y,
            paddle_2_y: self.paddle_2_y,
            ball_x: self.ball_x,
            ball_y: self.ball_y,
            size_y: self.size_y,
            size_x: self.size_x,
            points_1: self.points_1,
            points_2: self.points_2,
        }
    }

    fn reset_ball(&mut self, direction: BallDirection) {
        self.ball_x = self.size_x / 2 - super::BALL_SIZE / 2 - 1;
        self.ball_y = self.size_y / 2 - super::BALL_SIZE / 2 - 1;
        self.ball_direction = direction
    }

    fn scored(&mut self, id: usize) {
        match id {
            1 => {
                self.points_1 += 1;
            }
            2 => {
                self.points_2 += 1;
            }
            _ => {
                log::info!("Invalid Player");
            }
        }

        if self.points_1 >= 10 || self.points_2 >= 10 {
            self.points_1 = 0;
            self.points_2 = 0;
        }
    }

    fn paddle_hit(&mut self, paddle_y: u64) -> PaddleHit {
        if self.ball_y >= paddle_y.saturating_sub(super::BALL_SIZE - 1) && self.ball_y <= std::cmp::min(paddle_y + 2, self.size_y - 1) {
            return PaddleHit::Up;
        }

        if self.ball_y >= std::cmp::min(paddle_y + 3, self.size_y - 1) && self.ball_y <= std::cmp::min(paddle_y + 8, self.size_y - 1) {
            return PaddleHit::Center;
        }

        if self.ball_y >= std::cmp::min(paddle_y + 9, self.size_y - 1)
            && self.ball_y <= std::cmp::min(paddle_y + super::PADDLE_LEN - 1, self.size_y - 1)
        {
            return PaddleHit::Down;
        }

        PaddleHit::None
    }
}
