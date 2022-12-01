


#[test]
fn colorchord() {
    let step = (2.0_f64).powf(1.0 / 12.0);
    let mut f = 440.0_f64;
    println!("freq       hue");
    for _ in 0..13 {
        let hue = 360.0-(f / 440.0).log2() * 360.0;

        println!("{:.2} Hz   {:.0} deg", f, hue);

        f *= step;
    }
}


