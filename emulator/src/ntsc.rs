/** Portion of this code is derived from Zellyn Hunter
 *  https://observablehq.com/@zellyn/apple-ii-ntsc-emulation-openemulator-explainer
 */
use crate::video::{Rgb, Yuv};

const NTSC_SAMPLE_RATE: f32 = 14318181.818181818;
const NTSC_SUBCARRIER: f32 = 0.25;

fn real_idft(array: &[f32]) -> Vec<f32> {
    let size = array.len();
    let mut w = vec![0.0; size];

    for (i, v) in w.iter_mut().enumerate() {
        let omega = 2.0 * std::f32::consts::PI * (i as f32) / (size as f32);
        let sum = array.iter().enumerate().fold(0.0, |acc,(i,&value)| {
            acc + value * f32::cos(i as f32 * omega)
        });
        *v += sum;
        *v /= 1.0 / (size as f32);
    }
    w
}

fn chebyshev_window(n: usize, sidelobe: f32) -> Vec<f32> {
    let m = n - 1;
    let mut w = vec![0.0; m];
    let alpha = f32::cosh(f32::acosh(f32::powf(10.0, sidelobe / 20.0)) / m as f32);
    for (i,v) in w.iter_mut().enumerate() {
        let a = f32::abs(alpha * f32::cos(std::f32::consts::PI * i as f32 / m as f32));
        if a > 1.0 {
            *v = f32::powi(-1.0, i as i32) * f32::cosh(m as f32 * f32::acosh(a));
        } else {
            *v = f32::powi(-1.0, i as i32) * f32::cos(m as f32 * f32::acos(a));
        }
    }
    w = real_idft(&w);
    let mut t = vec![0.0; n];
    t[..usize::min(n, w.len())].copy_from_slice(&w[..usize::min(n, w.len())]);
    w = t;
    w[0] /= 2.0;
    w[n - 1] = w[0];

    let mut max = 0.0;
    for value in w.iter() {
        if f32::abs(*value) > max {
            max = f32::abs(*value)
        }
    }
    normalize(&scale(&w, 1.0 / max))
}

fn lanczos_window(n: usize, fc: f32) -> Vec<f32> {
    let mut v = vec![0.0; n];
    let fc = f32::min(fc, 0.5);
    let half_n = f32::floor(n as f32 / 2.0);
    v.iter_mut().enumerate().map(|(i,_)| {
        let x = 2.0 * std::f32::consts::PI * fc * (i as f32 - half_n);
        if x == 0.0 { 1.0 } else { f32::sin(x) / x }
    }).collect()
}

fn normalize(array: &[f32]) -> Vec<f32> {
    let sum = array.iter().fold(0.0, |acc, &value| acc + value);
    scale(array, 1.0 / sum)
}

fn scale(array: &[f32], scale: f32) -> Vec<f32> {
    array.iter().map(|value| value * scale).collect()
}

fn mul(array1: &[f32], array2: &[f32]) -> Vec<f32> {
    array1
        .iter()
        .enumerate()
        .map(|(i, value)| value * array2[i])
        .collect()
}

pub fn decoder_matrix(luma_bandwidth: f32, chroma_bandwidth: f32) -> Vec<Vec<f32>> {
    let y_bandwidth = luma_bandwidth / NTSC_SAMPLE_RATE;
    let u_bandwidth = chroma_bandwidth / NTSC_SAMPLE_RATE;
    let v_bandwidth = u_bandwidth;

    let w = chebyshev_window(17, 50.0);
    let wy = normalize(&mul(&w, &lanczos_window(17, y_bandwidth)));
    let wu = scale(&normalize(&mul(&w, &lanczos_window(17, u_bandwidth))), 2.0);
    let wv = scale(&normalize(&mul(&w, &lanczos_window(17, v_bandwidth))), 2.0);

    let decoder_matrix = vec![
        vec![wy[8], wu[8], wv[8]],
        vec![wy[7], wu[7], wv[7]],
        vec![wy[6], wu[6], wv[6]],
        vec![wy[5], wu[5], wv[5]],
        vec![wy[4], wu[4], wv[4]],
        vec![wy[3], wu[3], wv[3]],
        vec![wy[2], wu[2], wv[2]],
        vec![wy[1], wu[1], wv[1]],
        vec![wy[0], wu[0], wv[0]],
    ];

    eprintln!("cs={:?}", decoder_matrix);

    decoder_matrix
}

pub fn convert_chroma_to_yuv(x_pos: usize, dhgr: bool) -> Yuv {
    let p = 0.9083333;

    let phase = if dhgr {
        2.0 * std::f32::consts::PI * (NTSC_SUBCARRIER * (x_pos as f32 + 77.0 + 0.70) + p)
    } else {
        2.0 * std::f32::consts::PI * (NTSC_SUBCARRIER * (x_pos as f32 + 84.0 + 0.70) + p)
    };

    (1.0, f32::sin(phase), f32::cos(phase))
}

pub fn convert_yuv_to_rgb(yuv: Yuv) -> Rgb {
    let (y, u, v) = yuv;

    let r = ((y + 0.00000 * u + 1.13983 * v) * 255.0) as u8;
    let g = ((y - 0.39465 * u - 0.58060 * v) * 255.0) as u8;
    let b = ((y + 2.03211 * u + 0.00000 * v) * 255.0) as u8;

    [r, g, b]
}
