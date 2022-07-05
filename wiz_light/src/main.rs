mod wiz_rgb; 

fn main() {
    let ip = String::from("192.168.1.32");
    let mut wiz_light = wiz_rgb::setup_wiz_light(ip).unwrap();

    let green = wiz_rgb::RGBCW{
        r: 0, 
        g: 0, 
        b: 0, 
        c: 255, 
        w: 255
    };

    wiz_light.set_rgbcw(green);
    //wiz_light.turn_off(); 
    //wiz_light.turn_on();
}