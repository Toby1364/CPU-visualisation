#![windows_subsystem = "windows"]

use macroquad::prelude::*;
use macroquad::audio::*;
use rand::gen_range;
use std::collections::HashMap;
use std::fs;
use std::env;

fn smooth_step(x: f32) -> f32 {
    let n = 3.;
    x.powf(n)/(x.powf(n)+(1.0-x).powf(n))
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Byte {
    Unknown,
    Instruction,
    Register,
    Value,
    Pointer,
}

#[macroquad::main("")]
async fn main() {
    let args = env::args().collect::<Vec<_>>();
    
    let mut ram = [(0u8, Byte::Unknown); 0x1_0000];
    
    if args.len() > 1 {
        let file = fs::read_to_string(&args[1]).unwrap();
        let bytes = assemble(file);
        
        for i in 0..bytes.len() {
            ram[i].0 = bytes[i];
        }
    }

    let font = load_ttf_font_from_bytes(include_bytes!("Hack-Regular.ttf")).unwrap();

    let mut pointer: usize = 0;
    let mut target_pointer = 0;

    let mut reg_x: u16 = 0;
    let mut reg_y: u16 = 0;

    let mut cf = false;

    let instructions = HashMap::from([
        (0x00, ("NOP", "No operation, just increments the program counter, usually used for padding.")),
        (0x01, ("HLT", "Halt, halts the program.")),

        (0x10, ("MOV", "Move, copyies value from one register to another register.")),
        (0x11, ("LOD", "Load, loads value from memory to a register.")),
        (0x12, ("STO", "Store, stores value from a register to memory.")),
        (0x13, ("LDR", "Load by register, loads value from memory to a register using X as an address.")),
        (0x14, ("STR", "Store by register, stores value from a register to memory using X as an address.")),
        (0x15, ("SWP", "Swap, swaps X and Y.")),
        (0x16, ("LDI", "Load immediate, loads value into a register.")),

        (0x20, ("ADD", "Add, adds the Y value to X.")),
        (0x21, ("SUB", "Subtract, subtracts the Y value from X.")),
        (0x22, ("MUL", "Multiply, multiplies X by Y.")),
        (0x23, ("DIV", "Divide, divides X by Y.")),
        (0x24, ("MOD", "Modulo, divides X by Y and returns the remainder.")),

        (0x30, ("JMP", "Jump, jumps to a memory address.")),
        (0x31, ("JZ", "Jump if zero, jumps to a memory address if X is zero.")),
        (0x32, ("JNZ", "Jump if not zero, jumps to a memory address if X is not zero.")),
        (0x33, ("JC", "Jump if carry, jumps to a memory address if carry flag is set.")),
        (0x34, ("JNC", "Jump if not carry, jumps to a memory address if carry flag is not set.")),
        (0x35, ("JGE", "Jump if greater or equal, jumps to a memory address if X is greater or equal to Y.")),
        (0x36, ("JL", "Jump if less, jumps to a memory address if X is less than Y.")),

        (0x40, ("SCF", "Set carry flag, sets the carry flag.")),
        (0x41, ("CCF", "Clear carry flag, clears the carry flag.")),
    ]);

    let mut editing_index: Option<usize> = None;
    let mut editing_value = String::new();

    let mut offset = 0.;
    let mut anim = [0.; 3];

    let mut auto = false;
    let mut turbo: bool = false;
    let mut next = false;

    let mut halt = false;
    let mut played = false;

    let mut last_len = 0;

    let switch_sound = load_sound_from_bytes(include_bytes!("../sounds/switch.wav")).await.unwrap();
    set_sound_volume(switch_sound, 0.3);

    let next_sound = load_sound_from_bytes(include_bytes!("../sounds/next.wav")).await.unwrap();
    set_sound_volume(next_sound, 1.);

    let halt_sound = load_sound_from_bytes(include_bytes!("../sounds/halt.wav")).await.unwrap();
    set_sound_volume(halt_sound, 0.5);

    let type_sound = load_sound_from_bytes(include_bytes!("../sounds/type.wav")).await.unwrap();
    set_sound_volume(type_sound, 0.8);

    let mut delta: f32;

    let mut scale: f32;
    let mut frame: Vec2;
    loop {
        scale = screen_width() / 800.;
        if screen_height() < 450. * scale {scale = screen_height() / 450.}

        frame = Vec2{x: screen_width()/2. - 400. * scale, y: screen_height()/2. - 225. * scale};

        draw_rectangle(0., 0., frame.x, 450. *scale, BLACK);
        draw_rectangle(screen_width() - frame.x, 0., frame.x, screen_height(), BLACK);

        draw_rectangle(0., 0., screen_width(), frame.y, BLACK);
        draw_rectangle(0., screen_height() - frame.y, screen_width(), frame.y, BLACK);
        
        next_frame().await;
        clear_background(Color::from_hex(0x181818));
        
        delta = 1./get_fps() as f32;

        if pointer != target_pointer {
            if turbo {pointer = target_pointer}
            else {
                offset += delta*1.;
                if auto { offset += delta*1. }
                for i in 0..anim.len() {
                    anim[i] = 0.;
                }
                last_len = 0;
            }
        }
        if offset > 1. {
            offset = 0.;
            pointer = target_pointer;
        }

        if !auto && pointer == target_pointer && anim[0] < 1. {
            anim[0] += delta*2.;
        }

        if anim[0] >= 1. && anim[1] < 1. { anim[1] += delta*2. }
        if anim[1] >= 1. && anim[2] < 1. { anim[2] += delta*1. }
 
        if is_mouse_button_pressed(MouseButton::Left) {
            editing_value.clear();
            if mouse_position().0 > 210. *scale + frame.x && mouse_position().0 < (210. + 150.) *scale + frame.x && mouse_position().1 > 250. *scale + frame.y && mouse_position().1 < (250. + 70.) *scale + frame.y{
                editing_index = Some(0x100);
            }
            else if mouse_position().0 > 210. *scale + frame.x && mouse_position().0 < (210. + 150.) *scale + frame.x && mouse_position().1 > 320. *scale + frame.y && mouse_position().1 < (320. + 60.) *scale + frame.y{
                editing_index = Some(0x101);
            }
            else if mouse_position().0 > 210. *scale + frame.x && mouse_position().0 < (210. + 170.) *scale + frame.x && mouse_position().1 > 390. *scale + frame.y && mouse_position().1 < (390. + 40.) *scale + frame.y{
                editing_index = Some(0x102);
            }
            else if mouse_position().0 > 380. *scale + frame.x && mouse_position().0 < (380. + 90.) *scale + frame.x && mouse_position().1 > 390. *scale + frame.y && mouse_position().1 < (390. + 40.) *scale + frame.y{
                cf =! cf;
                play_sound_once(switch_sound);
            }
            else if mouse_position().0 > 458. *scale + frame.x && mouse_position().0 < (458. + 20.) *scale + frame.x && mouse_position().1 > 260. *scale + frame.y && mouse_position().1 < (260. + 36.) *scale + frame.y{
                auto =! auto;
                play_sound_once(switch_sound);
            }
            else if mouse_position().0 > 428. *scale + frame.x && mouse_position().0 < (428. + 20.) *scale + frame.x && mouse_position().1 > 260. *scale + frame.y && mouse_position().1 < (260. + 36.) *scale + frame.y{
                turbo =! turbo;
                play_sound_once(switch_sound);
            }
            else {
                editing_index = None;
            }
        }

        let thick = 6. *scale;

        for p in 0..0x1_0000 {
            let x = ((p as f32 - pointer as f32 + 4.)  * 47. - (smooth_step(offset) * 47. * (target_pointer as f32 - pointer as f32))) * scale + frame.x;
            let y = 120. * scale + frame.y;

            if x > -60. *scale + frame.x && x < screen_width() {
                if p <= 0xffff {
                    let n = ram[p].0;

                    draw_rectangle_lines(
                        x, 
                        y, 
                        50. *scale, 50. *scale, 
                        thick, 
                        WHITE
                    );            

                    let mut off = 0.;
                    if ram[p].1 != Byte::Unknown {
                        off = 8.;
                        draw_line(
                            x,
                            y + 20. *scale,
                            x + 50. *scale,
                            y + 20. *scale,
                            thick / 2.,
                            WHITE
                        );
                    }
                    if ram[p].1 == Byte::Instruction { if let Some(inst) = instructions.get(&n) {
                        draw_text_ex(
                            &format!("{}", inst.0), 
                            x+13. *scale, 
                            y+16. *scale, 
                            TextParams {
                                font,
                                font_size: (14. *scale) as u16,
                                color: RED,
                                ..Default::default()
                            }
                        );
                    }}

                    if ram[p].1 == Byte::Register {
                        let mut reg = "X";
                        if ram[p].0 != 00 { reg = "Y" }
                        draw_text_ex(
                            reg, 
                            x+20.5 *scale, 
                            y+16. *scale, 
                            TextParams {
                                font,
                                font_size: (14. *scale) as u16,
                                color: SKYBLUE,
                                ..Default::default()
                            }
                        );
                    }

                    if p > 0 && ram[p].1 == Byte::Value && ram[p-1].1 == Byte::Value {
                        draw_rectangle(
                            x, 
                            y + thick / 2., 
                            thick / 2., 
                            20. *scale - thick * 3. / 4., 
                            Color::from_hex(0x181818)
                        );

                        let n = (ram[p-1].0 as u16) << 8 | ram[p].0 as u16;
                        draw_text_ex(
                            &format!("{:04x}", n).to_uppercase(), 
                            x-15. *scale, 
                            y+16. *scale, 
                            TextParams {
                                font,
                                font_size: (14. *scale) as u16,
                                color: PURPLE,
                                ..Default::default()
                            }
                        );
                    }
                    if p > 0 && ram[p].1 == Byte::Pointer && ram[p-1].1 == Byte::Pointer {
                        draw_rectangle(
                            x, 
                            y + thick / 2., 
                            thick / 2., 
                            20. *scale - thick * 3. / 4., 
                            Color::from_hex(0x181818)
                        );

                        let n = (ram[p-1].0 as u16) << 8 | ram[p].0 as u16;
                        draw_text_ex(
                            &format!("{:04x}", n).to_uppercase(), 
                            x-15. *scale, 
                            y+16. *scale, 
                            TextParams {
                                font,
                                font_size: (14. *scale) as u16,
                                color: Color::from_hex(0xff8C00),
                                ..Default::default()
                            }
                        );
                    }

                    draw_text_ex(
                        &format!("{:02x}", n).to_uppercase(), 
                        x+14. *scale, 
                        y+(32.+off) *scale, 
                        TextParams {
                            font,
                            font_size: (18. *scale) as u16,
                            color: WHITE,
                            ..Default::default()
                        }
                    );

                    draw_text_ex(
                        &format!("{:04x}", p).to_uppercase(), 
                        x+10. *scale, 
                        y+60. *scale, 
                        TextParams {
                            font,
                            font_size: (15. *scale) as u16,
                            color: GRAY,
                            rotation: 0.8,
                            ..Default::default()
                        }
                    );

                    if is_mouse_button_pressed(MouseButton::Left) && mouse_position().0 > x && mouse_position().0 < x + 50.*scale && mouse_position().1 > y && mouse_position().1 < y + 50.*scale {
                        editing_index = Some(p);
                    }

                    if let Some(index) = editing_index {
                        if index == p {
                            draw_rectangle(
                                x+14. *scale, 
                                y+(18.+off) *scale, 
                                21.*scale, 
                                15.*scale, 
                                Color::from_hex(0x101010)
                            );

                            draw_text_ex(
                                &editing_value.to_uppercase(), 
                                x+14. *scale, 
                                y+(32.+off) *scale, 
                                TextParams {
                                    font,
                                    font_size: (18. *scale) as u16,
                                    color: WHITE,
                                    ..Default::default()
                                }
                            );

                            if editing_value.len() >= 2 {
                                if let Ok(n) = u8::from_str_radix(&editing_value, 16) {
                                    ram[p].0 = n;
                                }
                                editing_value.clear();
                                editing_index = None;
                            }
                        }
                    }
                }
            }
        }

        if let Some(c) = get_char_pressed() {
            editing_value.push(c);
        }

        if !auto && anim[0] > 0. {
            if let Some(inst) = instructions.get(&ram[pointer].0) {
                draw_rectangle_lines(
                    188. *scale + frame.x, 
                    (115. - 105. * smooth_step(anim[0])) *scale + frame.y, 
                    (50. + 188. * smooth_step(anim[1])) *scale, 
                    100. *scale * smooth_step(anim[0]), 
                    thick, 
                    Color::from_hex(0xff8C00)
                );

                let chars = inst.1.to_owned().chars().collect::<Vec<_>>();
                if (chars.len() as f32 * anim[2]) as usize <= chars.len() {
                    let desc_full: String = chars[0..(chars.len() as f32 * anim[2]) as usize].into_iter().collect::<String>();
                    let desc: Vec<String> = desc_full.split(" ").map(|s| s.to_owned()).collect();

                    if desc_full.len() > last_len {
                        play_sound_once(type_sound);
                        last_len = desc_full.len();
                    }

                    let mut lines: Vec<String> = vec![String::new()];

                    for w in 0..desc.len() {
                        let last = lines.len() - 1;
                        lines[last].push_str(&desc[w]);

                        if lines[last].len() > 24 {
                            lines[last] = lines[last].strip_suffix(&desc[w]).unwrap().to_owned();
                            lines.push(desc[w].clone());
                            lines[last+1].push(' ');
                        }
                        else {
                            lines[last].push(' ');
                        }
                    }

                    for l in 0..lines.len() {
                        draw_text_ex(
                            &format!("{}", lines[l].trim()), 
                            198. *scale + frame.x, 
                            (30. + 18.*l as f32) *scale + frame.y, 
                            TextParams {
                                font,
                                font_size: (15. *scale) as u16,
                                color: WHITE,
                                ..Default::default()
                            }
                        );
                    }
                }
            }
        }

        draw_rectangle_lines(
            188. *scale + frame.x, 
            250. *scale + frame.y, 
            300. *scale, 
            180. *scale, 
            thick, 
            WHITE
        );

        draw_rectangle_lines(
            188. *scale + frame.x, 
            120. *scale + frame.y, 
            50. *scale, 
            50. *scale, 
            thick, 
            Color::from_hex(0xff8C00)
        );

        draw_line(
            212. *scale + frame.x, 
            170. *scale + frame.y, 
            212. *scale + frame.x, 
            230. *scale + frame.y,  
            thick/2., 
            Color::from_hex(0xff8C00)
        );

        draw_line(
            168. *scale + frame.x, 
            230. *scale + frame.y, 
            168. *scale + frame.x, 
            408. *scale + frame.y,  
            thick/2., 
            Color::from_hex(0xff8C00)
        );

        draw_line(
            166.5 *scale + frame.x, 
            230. *scale + frame.y,  
            213.5 *scale + frame.x, 
            230. *scale + frame.y, 
            thick/2., 
            Color::from_hex(0xff8C00)
        );

        draw_line( 
            166.5 *scale + frame.x, 
            408. *scale + frame.y,
            205. *scale + frame.x, 
            408. *scale + frame.y,    
            thick/2., 
            Color::from_hex(0xff8C00)
        );

        draw_text_ex(
            &format!("Pointer: {}", format!("{:04x}",target_pointer).to_uppercase()), 
            210. *scale + frame.x, 
            414. *scale + frame.y,  
            TextParams {
                font,
                font_size: (20. *scale) as u16,
                color: WHITE,
                ..Default::default()
            }
        );

        draw_text_ex(
            &format!("X: {:04x}", reg_x).to_uppercase(), 
            210. *scale + frame.x, 
            305. *scale + frame.y,  
            TextParams {
                font,
                font_size: (30. *scale) as u16,
                color: WHITE,
                ..Default::default()
            }
        );

        draw_text_ex(
            &format!("Y: {:04x}", reg_y).to_uppercase(), 
            210. *scale + frame.x, 
            360. *scale + frame.y,  
            TextParams {
                font,
                font_size: (30. *scale) as u16,
                color: WHITE,
                ..Default::default()
            }
        );

        if let Some(ei) = editing_index {
            match ei {
                0x100 => {
                    draw_rectangle(
                        260. *scale + frame.x, 
                        275. *scale + frame.y, 
                        80. *scale, 
                        40. *scale, 
                        Color::from_hex(0x101010)
                    );
                    draw_text_ex(
                        &editing_value.to_uppercase(), 
                        265. *scale + frame.x, 
                        305. *scale + frame.y, 
                        TextParams {
                            font,
                            font_size: (30. *scale) as u16,
                            color: WHITE,
                            ..Default::default()
                        }
                    );
                    if editing_value.len() >= 4 {
                        if let Ok(n) = u16::from_str_radix(&editing_value, 16) {reg_x = n}
                        editing_value.clear();
                        editing_index = None;
                    }
                },
                0x101 => {
                    draw_rectangle(
                        260. *scale + frame.x, 
                        330. *scale + frame.y, 
                        80. *scale, 
                        40. *scale, 
                        Color::from_hex(0x101010)
                    );
                    draw_text_ex(
                        &editing_value.to_uppercase(), 
                        265. *scale + frame.x, 
                        360. *scale + frame.y, 
                        TextParams {
                            font,
                            font_size: (30. *scale) as u16,
                            color: WHITE,
                            ..Default::default()
                        }
                    );
                    if editing_value.len() >= 4 {
                        if let Ok(n) = u16::from_str_radix(&editing_value, 16) {reg_y = n}
                        editing_value.clear();
                        editing_index = None;
                    }
                }
                0x102 => {
                    draw_rectangle(
                        318. *scale + frame.x, 
                        395. *scale + frame.y, 
                        50. *scale, 
                        25. *scale, 
                        Color::from_hex(0x101010)
                    );
                    draw_text_ex(
                        &editing_value.to_uppercase(), 
                        318. *scale + frame.x, 
                        415. *scale + frame.y, 
                        TextParams {
                            font,
                            font_size: (20. *scale) as u16,
                            color: WHITE,
                            ..Default::default()
                        }
                    );
                    if editing_value.len() >= 4 {
                        if let Ok(n) = usize::from_str_radix(&editing_value, 16) {target_pointer = n}
                        editing_value.clear();
                        editing_index = None;
                    }
                }

                _ => {}
            }
        }
    
        draw_text_ex(
            "Carry flag:", 
            380. *scale + frame.x, 
            414. *scale + frame.y,  
            TextParams {
                font,
                font_size: (10. *scale) as u16,
                color: WHITE,
                ..Default::default()
            }
        );
        let mut color = Color::from_hex(0x550000);
        if cf {color = Color::from_hex(0xFF0000)}
        draw_circle(
            460. *scale + frame.x, 
            411. *scale + frame.y,
            8. *scale, 
            color
        );

        ram[pointer].1 = Byte::Instruction;
        let mut ip = target_pointer;
        for _ in 0..100 {
            ram[ip].1 = Byte::Instruction;
            match ram[ip].0 {
                /* NOP */ 0x00 => {ip += 1},
                /* HLT */ 0x01 => {break},

                
                /* MOV */ 0x10 => {
                    ram[ip + 1].1 = Byte::Register;
                    ip += 2;
                },
                /* LOD */ 0x11 => {
                    ram[ip + 1].1 = Byte::Register;
                    ram[ip + 2].1 = Byte::Pointer;
                    ram[ip + 3].1 = Byte::Pointer;
                    ip += 4;
                }
                /* STO */ 0x12 => {
                    ram[ip + 1].1 = Byte::Register;
                    ram[ip + 2].1 = Byte::Pointer;
                    ram[ip + 3].1 = Byte::Pointer;
                    ip += 4;
                }
                /* LDR */ 0x13 => {
                    ram[ip + 1].1 = Byte::Register;
                    ip += 2;
                }
                /* STR */ 0x14 => {
                    ram[ip + 1].1 = Byte::Register;
                    ip += 2;
                }
                /* SWP */ 0x15 => {
                    ip += 1;
                }
                /* LDI */ 0x16 => {
                    ram[ip + 1].1 = Byte::Register;
                    ram[ip + 2].1 = Byte::Value;
                    ram[ip + 3].1 = Byte::Value;
                    ip += 4;
                }

                /* ADD */ 0x20 => {
                    ip += 1;
                }
                /* SUB */ 0x21 => {
                    ip += 1;
                }
                /* MUL */ 0x22 => {
                    ip += 1;
                }
                /* DIV */ 0x23 => {
                    ip += 1;
                }
                /* MOD */ 0x24 => {
                    ip += 1;
                }

                /* JMP */ 0x30 => {
                    ram[ip + 1].1 = Byte::Pointer;
                    ram[ip + 2].1 = Byte::Pointer;
                    if 0.5 < gen_range(0., 1.) { ip += 3 }
                    else { ip = (ram[ip+1].0 as usize) << 8 | ram[ip+2].0 as usize }
                }
                /* JZ */ 0x31 => {
                    ram[ip + 1].1 = Byte::Pointer;
                    ram[ip + 2].1 = Byte::Pointer;

                    if 0.5 < gen_range(0., 1.) { ip += 3 }
                    else { ip = (ram[ip+1].0 as usize) << 8 | ram[ip+2].0 as usize }
                }
                /* JNZ */ 0x32 => {
                    ram[ip + 1].1 = Byte::Pointer;
                    ram[ip + 2].1 = Byte::Pointer;
                    
                    if 0.5 < gen_range(0., 1.) { ip += 3 }
                    else { ip = (ram[ip+1].0 as usize) << 8 | ram[ip+2].0 as usize }
                }
                /* JC */ 0x33 => {
                    ram[ip + 1].1 = Byte::Pointer;
                    ram[ip + 2].1 = Byte::Pointer;
                    
                    if 0.5 < gen_range(0., 1.) { ip += 3 }
                    else { ip = (ram[ip+1].0 as usize) << 8 | ram[ip+2].0 as usize }
                }
                /* JNC */ 0x34 => {
                    ram[ip + 1].1 = Byte::Pointer;
                    ram[ip + 2].1 = Byte::Pointer;
                    
                    if 0.5 < gen_range(0., 1.) { ip += 3 }
                    else { ip = (ram[ip+1].0 as usize) << 8 | ram[ip+2].0 as usize }
                }
                /* JGE */ 0x35 => {
                    ram[ip + 1].1 = Byte::Pointer;
                    ram[ip + 2].1 = Byte::Pointer;
                    
                    if 0.5 < gen_range(0., 1.) { ip += 3 }
                    else { ip = (ram[ip+1].0 as usize) << 8 | ram[ip+2].0 as usize }
                }
                /* JL */ 0x36 => {
                    ram[ip + 1].1 = Byte::Pointer;
                    ram[ip + 2].1 = Byte::Pointer;
                    
                    if 0.5 < gen_range(0., 1.) { ip += 3 }
                    else { ip = (ram[ip+1].0 as usize) << 8 | ram[ip+2].0 as usize }
                }

                /* SCF */ 0x40 => {
                    ip += 1;
                }
                /* CCF */ 0x41 => {
                    ip += 1;
                }


                _ => {break}
            }
        }

        if pointer == target_pointer && (auto || next) {
            next = false;
            played = false;

            match ram[pointer].0 {
                /* NOP */ 0x00 => {target_pointer += 1},
                /* HLT */ 0x01 => {halt = true},

                /* MOV */ 0x10 => {
                    if ram[pointer+1].0 == 0x00 {
                        reg_x = reg_y;
                    }
                    else {
                        reg_y = reg_x;
                    }
                    target_pointer += 2;
                },
                /* LOD */ 0x11 => {
                    let addr = (ram[pointer+2].0 as usize) << 8 | ram[pointer+3].0 as usize;
                    let n = (ram[addr].0 as u16) << 8 | ram[addr+1].0 as u16;

                    if ram[pointer+1].0 == 0x00 {
                        reg_x = n;
                    }
                    else {
                        reg_y = n;
                    }
                    target_pointer += 4;
                }
                /* STO */ 0x12 => {
                    let addr = (ram[pointer+2].0 as usize) << 8 | ram[pointer+3].0 as usize;

                    if ram[pointer+1].0 == 0x00 {
                        ram[addr].0 = (reg_x >> 8) as u8;
                        ram[addr+1].0 = reg_x as u8;
                    }
                    else {
                        ram[addr].0 = (reg_y >> 8) as u8;
                        ram[addr+1].0 = reg_y as u8;
                    }
                    target_pointer += 4;
                }
                /* LDR */ 0x13 => {
                    let n = (ram[reg_x as usize].0 as u16) << 8 | ram[(reg_x + 1) as usize].0 as u16;

                    if ram[pointer+1].0 == 0x00 {
                        reg_x = n;
                    }
                    else {
                        reg_y = n;
                    }
                    target_pointer += 2;
                }
                /* STR */ 0x14 => {
                    if ram[pointer+1].0 == 0x00 {
                        ram[reg_x as usize].0 = (reg_x >> 8) as u8;
                        ram[(reg_x+1) as usize].0 = reg_x as u8;
                    }
                    else {
                        ram[reg_x as usize].0 = (reg_y >> 8) as u8;
                        ram[(reg_x+1) as usize].0 = reg_y as u8;
                    }
                    target_pointer += 2;
                }
                /* SWP */ 0x15 => {
                    let tmp = reg_x;
                    reg_x = reg_y;
                    reg_y = tmp;
                    target_pointer += 1;
                }
                /* LDI */ 0x16 => {
                    let n = (ram[pointer+2].0 as u16) << 8 | ram[pointer+3].0 as u16;
                    
                    if ram[pointer+1].0 == 0x00 {
                        reg_x = n;
                    }
                    else {
                        reg_y = n;
                    }
                    target_pointer += 4;
                }

                /* ADD */ 0x20 => {
                    reg_x = reg_x.wrapping_add(reg_y);
                    target_pointer += 1;

                    if reg_x as u32 + reg_y as u32 > u16::MAX as u32 { cf = true }
                }
                /* SUB */ 0x21 => {
                    reg_x = reg_x.wrapping_sub(reg_y);
                    target_pointer += 1;

                    if reg_x as u32 - reg_y as u32 > u16::MIN as u32 { cf = true }
                }
                /* MUL */ 0x22 => {
                    reg_x *= reg_y;
                    target_pointer += 1;
                }
                /* DIV */ 0x23 => {
                    reg_x /= reg_y;
                    target_pointer += 1;
                }
                /* MOD */ 0x24 => {
                    reg_x %= reg_y;
                    target_pointer += 1;
                }

                /* JMP */ 0x30 => {
                    target_pointer = (ram[pointer+1].0 as usize) << 8 | ram[pointer+2].0 as usize;
                }
                /* JZ */ 0x31 => {
                    if reg_x == 0 {
                        target_pointer = (ram[pointer+1].0 as usize) << 8 | ram[pointer+2].0 as usize;
                    }
                    else {
                        target_pointer += 3;
                    }
                }
                /* JNZ */ 0x32 => {
                    if reg_x != 0 {
                        target_pointer = (ram[pointer+1].0 as usize) << 8 | ram[pointer+2].0 as usize;
                    }
                    else {
                        target_pointer += 3;
                    }
                }
                /* JC */ 0x33 => {
                    if cf {
                        target_pointer = (ram[pointer+1].0 as usize) << 8 | ram[pointer+2].0 as usize;
                    }
                    else {
                        target_pointer += 3;
                    }
                }
                /* JNC */ 0x34 => {
                    if !cf {
                        target_pointer = (ram[pointer+1].0 as usize) << 8 | ram[pointer+2].0 as usize;
                    }
                    else {
                        target_pointer += 3;
                    }
                }
                /* JGE */ 0x35 => {
                    if reg_x >= reg_y {
                        target_pointer = (ram[pointer+1].0 as usize) << 8 | ram[pointer+2].0 as usize;
                    }
                    else {
                        target_pointer += 3;
                    }
                }
                /* JL */ 0x36 => {
                    if reg_x < reg_y {
                        target_pointer = (ram[pointer+1].0 as usize) << 8 | ram[pointer+2].0 as usize;
                    }
                    else {
                        target_pointer += 3;
                    }
                }

                /* SCF */ 0x40 => {
                    cf = true;
                    target_pointer += 1;
                }
                /* CCF */ 0x41 => {
                    cf = false;
                    target_pointer += 1;
                }

                _ => {}
            }
        }

        if offset > 0.7 && !played && !turbo {
            if ram[target_pointer].0 == 1 { play_sound_once(halt_sound) }
            else { play_sound_once(next_sound) }
            played = true;
        }

        if pointer == target_pointer && is_key_down(KeyCode::Space) {
            next = true;
        }

        { // Auto switch
            draw_rectangle(
                458. *scale + frame.x, 
                260. *scale + frame.y, 
                20. * scale, 
                36. * scale,  
                Color::from_hex(0x080808)
            );

            draw_rectangle_lines(
                458. *scale + frame.x, 
                260. *scale + frame.y, 
                20. * scale, 
                36. * scale, 
                thick, 
                WHITE
            );
            draw_rectangle(
                458. *scale + frame.x + thick /2., 
                (260. + (16. * !auto as u32 as f32)) *scale + frame.y + thick / 2., 
                20. * scale - thick, 
                20. * scale - thick, 
                Color::from_hex(0x101010),
            );

            let mut color = Color::from_hex(0x550000);
            if auto {color = Color::from_hex(0xFF0000)}

            draw_rectangle(
                458. *scale + frame.x + thick * 3. / 4., 
                (260. + 16. * !auto as u32 as f32) *scale + frame.y + thick * 3. / 4., 
                20. * scale - thick * 1.5, 
                20. * scale - thick * 1.5, 
                color,
            );
        }
        { // Turbo Switch
            draw_rectangle(
                428. *scale + frame.x, 
                260. *scale + frame.y, 
                20. * scale, 
                36. * scale,  
                Color::from_hex(0x080808)
            );

            draw_rectangle_lines(
                428. *scale + frame.x, 
                260. *scale + frame.y, 
                20. * scale, 
                36. * scale, 
                thick, 
                WHITE
            );
            draw_rectangle(
                428. *scale + frame.x + thick /2., 
                (260. + (16. * !turbo as u32 as f32)) *scale + frame.y + thick / 2., 
                20. * scale - thick, 
                20. * scale - thick, 
                Color::from_hex(0x101010),
            );

            let mut color = Color::from_hex(0x300030);
            if turbo {color = Color::from_hex(0xB000F0)}

            draw_rectangle(
                428. *scale + frame.x + thick * 3. / 4., 
                (260. + 16. * !turbo as u32 as f32) *scale + frame.y + thick * 3. / 4., 
                20. * scale - thick * 1.5, 
                20. * scale - thick * 1.5, 
                color,
            );
        }

        draw_rectangle_lines(
            520. *scale + frame.x, 
            250. *scale + frame.y, 
            270. *scale, 
            180. *scale, 
            thick, 
            WHITE
        );

        for y in 0..16 {
            for x in 0..8 {
                draw_text_ex(
                    &format!("{:02x}", ram[0xFF00 + y*16 + x].0).to_uppercase(), 
                    (530. + x as f32*15.) *scale + frame.x, 
                    (269. + y as f32*10.) *scale + frame.y, 
                    TextParams {
                        font,
                        font_size: (11. *scale) as u16,
                        color: WHITE,
                        ..Default::default()
                    }
                );
                draw_text_ex(
                    &format!("{:02x}", ram[0xFF00 + y*16 + x+8].0).to_uppercase(), 
                    (663. + x as f32*15.) *scale + frame.x, 
                    (269. + y as f32*10.) *scale + frame.y, 
                    TextParams {
                        font,
                        font_size: (11. *scale) as u16,
                        color: WHITE,
                        ..Default::default()
                    }
                );
            }
        }
    }
}

fn assemble(mut asm: String) -> Vec<u8> {
    let mut bytes = Vec::new();

    // resolve labels
    let mut labels: Vec<(String, usize)> = Vec::new();

    let mut addr = 0;
    for line in asm.lines() {
        if line.is_empty() {continue}
        let mut inst = line.split(' ').collect::<Vec<&str>>()[0];

        if inst.ends_with(':') {
            inst = inst.trim_end_matches(':');
            labels.push((inst.to_owned(), addr));
        }
        
        match inst.to_uppercase().as_str() {
            "NOP" => { addr += 1 }
            "HLT" => { addr += 1 }

            "MOV" => { addr += 2 }
            "LOD" => { addr += 4 }
            "STO" => { addr += 4 }
            "LDR" => { addr += 2 }
            "STR" => { addr += 2 }
            "SWP" => { addr += 1 }
            "LDI" => { addr += 4 }

            "ADD" => { addr += 1 }
            "SUB" => { addr += 1 }
            "MUL" => { addr += 1 }
            "DIV" => { addr += 1 }
            "MOD" => { addr += 1 }

            "JMP" => { addr += 3 }
            "JZ" => { addr += 3 }
            "JNZ" => { addr += 3 }
            "JC" => { addr += 3 }
            "JNC" => { addr += 3 }
            "JGE" => { addr += 3 }
            "JL" => { addr += 3 }

            "SCF" => { addr += 1 }
            "CCF" => { addr += 1 }

            _ => {}
        }
    }

    for label in labels {
        asm = asm.replace(&label.0, &format!("{:04x}", label.1));
    }
    asm = asm.replace("x", "00");
    asm = asm.replace("y", "01");


    for line in asm.lines() {
        if line.is_empty() {continue}

        let line = line.trim();
        let mut line = line.split(' ');
        let inst = line.next().unwrap();
        let args: Vec<u16> = line.map(|s| u16::from_str_radix(s, 16).unwrap()).collect();

        match inst.to_uppercase().as_str() {
            "NOP" => { bytes.push(0x00) }
            "HLT" => { bytes.push(0x01) }

            "MOV" => { 
                bytes.push(0x10);      
                bytes.push(args[0] as u8);          
            }
            "LOD" => { 
                bytes.push(0x11);
                bytes.push(args[0] as u8);
                bytes.push((args[1] >> 8) as u8);
                bytes.push(args[1] as u8);
            }
            "STO" => { 
                bytes.push(0x12);
                bytes.push(args[0] as u8);
                bytes.push((args[1] >> 8) as u8);
                bytes.push(args[1] as u8);
            }
            "LDR" => { 
                bytes.push(0x13);
                bytes.push(args[0] as u8);
            }
            "STR" => { 
                bytes.push(0x14);
                bytes.push(args[0] as u8);
            }
            "SWP" => { bytes.push(0x15) }
            "LDI" => { 
                bytes.push(0x16);
                bytes.push(args[0] as u8);
                bytes.push((args[1] >> 8) as u8);
                bytes.push(args[1] as u8);
            }

            "ADD" => { bytes.push(0x20) }
            "SUB" => { bytes.push(0x21) }
            "MUL" => { bytes.push(0x22) }
            "DIV" => { bytes.push(0x23) }
            "MOD" => { bytes.push(0x24) }

            "JMP" => {     
                bytes.push(0x30);
                bytes.push((args[0] >> 8) as u8);
                bytes.push(args[0] as u8);
            }
            "JZ"  => {     
                bytes.push(0x31);
                bytes.push((args[0] >> 8) as u8);
                bytes.push(args[0] as u8);
            }
            "JNZ" => {     
                bytes.push(0x32);
                bytes.push((args[0] >> 8) as u8);
                bytes.push(args[0] as u8);
            }
            "JC"  => {     
                bytes.push(0x33);
                bytes.push((args[0] >> 8) as u8);
                bytes.push(args[0] as u8);
            }
            "JNC" => {     
                bytes.push(0x34); 
                bytes.push((args[0] >> 8) as u8);
                bytes.push(args[0] as u8);
            }
            "JGE" => {     
                bytes.push(0x35); 
                bytes.push((args[0] >> 8) as u8);
                bytes.push(args[0] as u8);
            }
            "JL"  => {     
                bytes.push(0x36); 
                bytes.push((args[0] >> 8) as u8);
                bytes.push(args[0] as u8);
            }

            "SCF" => { bytes.push(0x37) }
            "CCF" => { bytes.push(0x38) }

            _ => {}
        }
    }
    bytes
}
