/// 计算向量 L2 范数
pub fn vector_norm(v: &[f32]) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}

/// 计算余弦相似度（调用方需提供 query_norm 以避免重复计算）
pub fn cosine_similarity(a: &[f32], b: &[f32], a_norm: f32) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let b_norm = vector_norm(b);
    if b_norm == 0.0 {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    dot / (a_norm * b_norm)
}
