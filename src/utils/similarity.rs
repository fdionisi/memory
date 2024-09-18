use crate::domain::embedding::Embedding;

pub fn cosine_similarity(a: &Embedding, b: &Embedding) -> f32 {
    let a_vec = a.to_vec();
    let b_vec = b.to_vec();

    let dot_product: f32 = a_vec.iter().zip(b_vec.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a_vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b_vec.iter().map(|x| x * x).sum::<f32>().sqrt();

    dot_product / (magnitude_a * magnitude_b)
}
