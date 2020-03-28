use rand::prelude::*;
use super::GenCtx;

pub struct SellOrder {
    pub quantity: f32,
    pub price: f32,

    // The money returned to the seller
    pub q_sold: f32,
}

pub struct BuyOrder {
    quantity: f32,
    max_price: f32,
}

#[derive(Clone, Debug)]
pub struct Belief {
    pub price: f32,
    pub confidence: f32,
}

impl Belief {
    pub fn choose_price(&self, ctx: &mut GenCtx<impl Rng>) -> f32 {
        self.price + ctx.rng.gen_range(-1.0, 1.0) * self.confidence
    }

    pub fn update_buyer(&mut self, years: f32, new_price: f32) {
        if (self.price - new_price).abs() < self.confidence {
            self.confidence *= 0.8;
        } else {
            self.price += (new_price - self.price) * 0.5; // TODO: Make this vary with `years`
            self.confidence = (self.price - new_price).abs();
        }
    }

    pub fn update_seller(&mut self, proportion: f32) {
        self.price *= 1.0 + (proportion - 0.5) * 0.25;
        self.confidence /= 1.0 + (proportion - 0.5) * 0.25;
    }
}

pub fn buy_units<'a>(
    ctx: &mut GenCtx<impl Rng>,
    sellers: impl Iterator<Item=&'a mut SellOrder>,
    max_quantity: f32,
    max_price: f32,
    max_spend: f32,
) -> (f32, f32) {
    let mut sell_orders = sellers
        .filter(|so| so.quantity > 0.0)
        .collect::<Vec<_>>();
    // Sort sell orders by price, cheapest first
    sell_orders.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap_or_else(|| panic!("{} and {}", a.price, b.price)));

    let mut quantity = 0.0;
    let mut spent = 0.0;

    for order in sell_orders {
        if
            quantity >= max_quantity || // We've purchased enough
            spent >= max_spend || // We've spent enough
            order.price > max_price // Price is too high
        {
            break;
        } else {
            let q = (max_quantity - quantity)
                .min(order.quantity - order.q_sold)
                .min((max_spend - spent) / order.price);
            order.q_sold += q;
            quantity += q;
            spent += q * order.price;
        }
    }

    (quantity, spent)
}
