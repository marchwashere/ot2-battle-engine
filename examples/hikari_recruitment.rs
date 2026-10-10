use ot2_battle_engine::{CrtRng,field_escape::route_player,hikari_recruitment};
fn main()->Result<(),Box<dyn std::error::Error>> {
    let seed=std::env::args().nth(1).map(|s|if let Some(s)=s.strip_prefix("0x") {
        u32::from_str_radix(s,16)
    } else {s.parse()}).transpose()?.unwrap_or(0);
    let players=[(7,1530),(6,1467),(8,0),(4,0)].into_iter()
        .map(|(id,exp)|route_player(id,exp)).collect::<Result<Vec<_>,_>>()?;
    let p=hikari_recruitment::predict(CrtRng::new(seed),&players,&Default::default())?;
    println!("{}",serde_json::to_string_pretty(&p)?);
    Ok(())
}
