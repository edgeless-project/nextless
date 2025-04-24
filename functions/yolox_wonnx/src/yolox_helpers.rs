// SPDX-FileCopyrightText: © 2021 Xavier Tao, Tommy van der Vorst & WONNX contributors
// SPDX-FileCopyrightText: © 2025 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
// YoloX WONNX Example adapted from Kazuki Ikemori's fork of WONNX https://github.com/kadu-v/wonnx/blob/dev/slice/wonnx/examples/yolox_nano.rs.
// The changes in the fork were offered to WONNX by Kazuki Ikemori: https://github.com/webonnx/wonnx/pull/214.

use image::Pixel;

/*-----------------------------------------------------------------------------
 Post processing
--------------------------------------------------------------------------------*/

// Diabled as imageproc requires bindgen
// pub(crate) fn draw_detections(image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, detections: Vec<(String, f32, f32, f32, f32, f32)>) {
//     for (_class, _score, x0, y0, x1, y1) in detections.iter() {
//         draw_rect(image, *x0, *y0, *x1, *y1);
//     }
// }
// fn draw_rect(image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, x1: f32, y1: f32, x2: f32, y2: f32) {
//     let x1 = x1 as u32;
//     let y1 = y1 as u32;
//     let x2 = x2 as u32;
//     let y2 = y2 as u32;
//     let rect = imageproc::rect::Rect::at(x1 as i32, y1 as i32).of_size(x2 - x1 as u32, (y2 - y1) as u32);
//     imageproc::drawing::draw_hollow_rect_mut(image, rect, Rgb([255, 0, 0]));
// }

fn calc_loc(positions: &Vec<(f32, f32, f32, f32)>) -> Vec<(f32, f32, f32, f32)> {
    let mut locs = vec![];

    // calc girds
    let (h, w) = (416, 416);
    let strides = vec![8, 16, 32];
    let mut h_grids = vec![];
    let mut w_grids = vec![];

    for stride in strides.iter() {
        let mut h_grid = vec![0.0; h / stride];
        let mut w_grid = vec![0.0; w / stride];

        for i in 0..h / stride {
            h_grid[i] = i as f32;
        }
        for i in 0..w / stride {
            w_grid[i] = i as f32;
        }
        h_grids.push(h_grid);
        w_grids.push(w_grid);
    }
    let acc = vec![0, 52 * 52, 52 * 52 + 26 * 26, 52 * 52 + 26 * 26 + 13 * 13];

    for (i, stride) in strides.iter().enumerate() {
        let h_grid = &h_grids[i];
        let w_grid = &w_grids[i];
        let idx = acc[i];

        for (i, y) in h_grid.iter().enumerate() {
            for (j, x) in w_grid.iter().enumerate() {
                let p = idx + i * w / stride + j;
                let (px, py, pw, ph) = positions[p];
                let (x, y) = ((x + px) * *stride as f32, (y + py) * *stride as f32);
                let (ww, hh) = (pw.exp() * *stride as f32, ph.exp() * *stride as f32);
                let loc = (x - ww / 2.0, y - hh / 2.0, x + ww / 2.0, y + hh / 2.0);
                locs.push(loc);
            }
        }
    }
    locs
}

fn non_max_suppression(
    boxes: &Vec<(f32, f32, f32, f32)>,
    scores: &Vec<f32>,
    score_threshold: f32,
    iou_threshold: f32,
) -> Vec<(usize, (f32, f32, f32, f32))> {
    let mut new_boxes = vec![];
    let mut sorted_indices = (0..boxes.len()).collect::<Vec<_>>();
    sorted_indices.sort_by(|a, b| scores[*a].partial_cmp(&scores[*b]).unwrap());

    while let Some(last) = sorted_indices.pop() {
        let mut remove_list = vec![];
        let score = scores[last];
        let bbox = boxes[last];
        let mut numerator = (bbox.0 * score, bbox.1 * score, bbox.2 * score, bbox.3 * score);
        let mut denominator = score;

        for i in 0..sorted_indices.len() {
            let idx = sorted_indices[i];
            let (x1, y1, x2, y2) = boxes[idx];
            let (x1_, y1_, x2_, y2_) = boxes[last];
            let box1_area = (x2 - x1) * (y2 - y1);

            let inter_x1 = x1.max(x1_);
            let inter_y1 = y1.max(y1_);
            let inter_x2 = x2.min(x2_);
            let inter_y2 = y2.min(y2_);
            let inter_w = (inter_x2 - inter_x1).max(0.0);
            let inter_h = (inter_y2 - inter_y1).max(0.0);
            let inter_area = inter_w * inter_h;
            let area1 = (x2 - x1) * (y2 - y1);
            let area2 = (x2_ - x1_) * (y2_ - y1_);
            let union_area = area1 + area2 - inter_area;
            let iou = inter_area / union_area;

            if scores[idx] < score_threshold {
                remove_list.push(i);
            } else if iou > iou_threshold {
                remove_list.push(i);
                let w = scores[idx] * iou;
                numerator = (
                    numerator.0 + boxes[idx].0 * w,
                    numerator.1 + boxes[idx].1 * w,
                    numerator.2 + boxes[idx].2 * w,
                    numerator.3 + boxes[idx].3 * w,
                );
                denominator += w;
            } else if inter_area / box1_area > 0.7 {
                remove_list.push(i);
            }
        }
        for i in remove_list.iter().rev() {
            sorted_indices.remove(*i);
        }
        let new_bbox = (
            numerator.0 / denominator,
            numerator.1 / denominator,
            numerator.2 / denominator,
            numerator.3 / denominator,
        );
        new_boxes.push((last, new_bbox));
    }
    new_boxes
}

pub(crate) fn post_process(preds: &[f32]) -> Vec<(String, f32, f32, f32, f32, f32)> {
    let labels = get_coco_labels();
    let mut positions = vec![];
    let mut classes = vec![];
    let mut objectnesses = vec![];
    for i in 0..3549 {
        let offset = i * 85;
        let objectness = preds[offset + 4];

        let (class, score) = preds[offset + 5..offset + 85]
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();
        let class = labels[class].clone();
        let x1 = preds[offset];
        let y1 = preds[offset + 1];
        let x2 = preds[offset + 2];
        let y2 = preds[offset + 3];
        classes.push((class, score));
        positions.push((x1, y1, x2, y2));
        objectnesses.push(objectness);
    }

    let locs = calc_loc(&positions);

    let mut result = vec![];
    // filter by objectness
    let indices = non_max_suppression(&locs, &objectnesses, 0.5, 0.3);
    for bbox in indices {
        let (i, (x, y, w, h)) = bbox;
        let (class, &score) = &classes[i];
        result.push((class.clone(), score, x, y, w, h));
    }
    result
}

/*-----------------------------------------------------------------------------
 Pre processing
--------------------------------------------------------------------------------*/

pub(crate) fn resize_image(image: image::ImageBuffer<image::Rgb<u8>, Vec<u8>>) -> image::ImageBuffer<image::Rgb<u8>, Vec<u8>> {
    let image = pad_image(image);
    image::imageops::resize(&image, 416, 416, image::imageops::FilterType::Nearest)
}

pub(crate) fn convert_image_to_net_input(input_image: &image::ImageBuffer<image::Rgb<u8>, Vec<u8>>) -> Vec<f32> {
    let mut image = vec![0.0; 3 * 416 * 416];
    for j in 0..416 {
        for i in 0..416 {
            let pixel = input_image.get_pixel(i as u32, j as u32);
            let channels = pixel.channels();
            for c in 0..3 {
                image[c * 416 * 416 + j * 416 + i] = channels[c] as f32;
            }
        }
    }
    image
}

fn pad_image(image: image::ImageBuffer<image::Rgb<u8>, Vec<u8>>) -> image::ImageBuffer<image::Rgb<u8>, Vec<u8>> {
    let (width, height) = image.dimensions();
    let target_size = if width > height { width } else { height };
    let mut new_image = image::ImageBuffer::new(target_size as u32, target_size as u32);
    let x_offset = (target_size as u32 - width) / 2;
    let y_offset = (target_size as u32 - height) / 2;
    for j in 0..height {
        for i in 0..width {
            let pixel = image.get_pixel(i, j);
            new_image.put_pixel(i + x_offset, j + y_offset, *pixel);
        }
    }
    new_image
}

fn get_coco_labels() -> Vec<String> {
    // Download the ImageNet class labels, matching SqueezeNet's classes.
    let lables = include_str!("../data/coco-classes.txt").to_string();
    lables.split("\n").map(|s| s.to_string()).collect()
}
