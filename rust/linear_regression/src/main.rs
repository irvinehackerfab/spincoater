use std::env;

use color_eyre::eyre::{OptionExt, Result, eyre};
use host_tui::app::state::MotionProfileState;
use linreg::linear_regression;
use num::{FromPrimitive, ToPrimitive, rational::Ratio};

fn main() -> Result<()> {
    let path = rfd::FileDialog::new()
        .add_filter("CSV", &["csv"])
        .set_directory(env::current_dir()?)
        .set_title("Please choose a motor data CSV file.")
        .pick_file()
        .ok_or_eyre("File not selected.")?;

    let file = csv::Reader::from_path(path)?;
    let mut rpm_values = Vec::new();
    let mut duty_cycle_values = Vec::new();
    for result in file.into_deserialize() {
        let state: MotionProfileState = result?;
        rpm_values.push(state.current_rpm);
        duty_cycle_values.push(*state.duty_cycle);
    }

    let (slope, intercept): (f64, f64) =
        linear_regression(&rpm_values, &duty_cycle_values).map_err(|error| eyre!(error))?;

    let slope = Ratio::<u16>::from_f64(slope).expect("The slope should be a valid ratio");
    let intercept = intercept
        .to_u16()
        .expect("The intercept should fit in a u16");
    println!("RPM_TO_DUTY_NUMERATOR: {}", slope.numer());
    println!("RPM_TO_DUTY_DENOMINATOR: {}", slope.denom());
    println!("RPM_TO_DUTY_INTERCEPT: {intercept}");

    Ok(())
}
