use crate::{
    common::error::{decode_error, error_result, is_error_result},
    dict_builder::zdict::{evaluate_dictionary_score, parse_samples, train_dictionary},
    ffi::types::{ZDICT_fastCover_params_t, ZDICT_params_t, ZSTD_ErrorCode},
};
use core::ffi::{c_uint, c_void};

const FASTCOVER_MAX_ACCEL: c_uint = 10;

fn normalize_params(mut parameters: ZDICT_fastCover_params_t) -> ZDICT_fastCover_params_t {
    if parameters.k == 0 {
        parameters.k = 64;
    }
    if parameters.d == 0 {
        parameters.d = 8;
    }
    if parameters.f == 0 {
        parameters.f = 20;
    }
    if parameters.steps == 0 {
        parameters.steps = 4;
    }
    if parameters.accel == 0 {
        parameters.accel = 1;
    }
    if parameters.splitPoint == 0.0 {
        parameters.splitPoint = 0.75;
    }
    parameters
}

fn validate_params(
    parameters: ZDICT_fastCover_params_t,
) -> Result<ZDICT_fastCover_params_t, ZSTD_ErrorCode> {
    let parameters = normalize_params(parameters);
    if !(parameters.splitPoint > 0.0 && parameters.splitPoint <= 1.0) {
        return Err(ZSTD_ErrorCode::ZSTD_error_parameter_outOfBound);
    }
    if parameters.accel > FASTCOVER_MAX_ACCEL {
        return Err(ZSTD_ErrorCode::ZSTD_error_parameter_outOfBound);
    }
    Ok(parameters)
}

fn zparams(parameters: ZDICT_fastCover_params_t) -> ZDICT_params_t {
    parameters.zParams
}

fn split_samples<'a>(
    samples: &'a [&'a [u8]],
    split_point: f64,
) -> (&'a [&'a [u8]], &'a [&'a [u8]]) {
    let split = if split_point <= 0.0 {
        0.75
    } else {
        split_point
    }
    .clamp(0.0, 1.0);
    let train_end = ((samples.len() as f64) * split)
        .round()
        .clamp(1.0, samples.len() as f64) as usize;
    let train = &samples[..train_end];
    let test = if train_end >= samples.len() {
        train
    } else {
        &samples[train_end..]
    };
    (train, test)
}

fn train_and_score(
    dict_buffer_capacity: usize,
    samples_buffer: *const c_void,
    samples_sizes: *const usize,
    nb_samples: c_uint,
    preferred_segment_size: usize,
    params: ZDICT_fastCover_params_t,
    validation_samples: &[&[u8]],
) -> Result<(usize, usize), ZSTD_ErrorCode> {
    let mut dictionary = vec![0u8; dict_buffer_capacity];
    let size = train_dictionary(
        dictionary.as_mut_ptr().cast(),
        dictionary.len(),
        samples_buffer,
        samples_sizes,
        nb_samples,
        preferred_segment_size,
        zparams(params),
        params.shrinkDict != 0,
    );
    if is_error_result(size) {
        return Err(decode_error(size));
    }
    dictionary.truncate(size);
    let score = evaluate_dictionary_score(
        &dictionary,
        validation_samples,
        params.zParams.compressionLevel,
    )?;
    Ok((score, size))
}

#[no_mangle]
pub extern "C" fn ZDICT_trainFromBuffer_fastCover(
    dictBuffer: *mut c_void,
    dictBufferCapacity: usize,
    samplesBuffer: *const c_void,
    samplesSizes: *const usize,
    nbSamples: c_uint,
    parameters: ZDICT_fastCover_params_t,
) -> usize {
    let parameters = match validate_params(parameters) {
        Ok(parameters) => parameters,
        Err(code) => return error_result(code),
    };
    let preferred_segment_size = parameters.k.max(parameters.f).max(parameters.d) as usize;
    train_dictionary(
        dictBuffer,
        dictBufferCapacity,
        samplesBuffer,
        samplesSizes,
        nbSamples,
        preferred_segment_size,
        zparams(parameters),
        parameters.shrinkDict != 0,
    )
}

#[no_mangle]
pub extern "C" fn ZDICT_optimizeTrainFromBuffer_fastCover(
    dictBuffer: *mut c_void,
    dictBufferCapacity: usize,
    samplesBuffer: *const c_void,
    samplesSizes: *const usize,
    nbSamples: c_uint,
    parameters: *mut ZDICT_fastCover_params_t,
) -> usize {
    let Some(parameters) = (unsafe { parameters.as_mut() }) else {
        return error_result(ZSTD_ErrorCode::ZSTD_error_GENERIC);
    };
    let base = match validate_params(*parameters) {
        Ok(parameters) => parameters,
        Err(code) => return error_result(code),
    };
    let samples = match parse_samples(samplesBuffer, samplesSizes, nbSamples) {
        Ok(samples) => samples,
        Err(code) => return error_result(code),
    };
    let (training_samples, validation_samples) = split_samples(&samples, base.splitPoint);
    let mut best = base;
    let mut best_score = usize::MAX;
    let mut best_size = usize::MAX;

    let k_min = if parameters.k == 0 { 50 } else { parameters.k };
    let k_max = if parameters.k == 0 {
        (dictBufferCapacity as u32).clamp(k_min, 2000)
    } else {
        parameters.k
    };
    let d_min = if parameters.d == 0 { 6 } else { parameters.d };
    let d_max = if parameters.d == 0 { 8 } else { parameters.d };
    let steps = if parameters.steps == 0 {
        40
    } else {
        parameters.steps
    }
    .max(1);
    let step_size = ((k_max.saturating_sub(k_min)) / steps).max(1);

    for d in (d_min..=d_max).step_by(2) {
        let mut k = k_min.max(d);
        while k <= k_max {
            let mut candidate = *parameters;
            candidate.k = k;
            candidate.d = d;
            candidate.steps = steps;
            candidate = normalize_params(candidate);
            let preferred_segment_size = candidate.k.max(candidate.f).max(candidate.d) as usize;
            match train_and_score(
                dictBufferCapacity,
                samplesBuffer,
                samplesSizes,
                training_samples.len() as c_uint,
                preferred_segment_size,
                candidate,
                validation_samples,
            ) {
                Ok((score, size))
                    if score < best_score || (score == best_score && size < best_size) =>
                {
                    best = candidate;
                    best_score = score;
                    best_size = size;
                }
                Ok(_) => {}
                Err(code) => return error_result(code),
            }
            if k_max - k < step_size {
                break;
            }
            k += step_size;
        }
    }

    *parameters = best;
    ZDICT_trainFromBuffer_fastCover(
        dictBuffer,
        dictBufferCapacity,
        samplesBuffer,
        samplesSizes,
        nbSamples,
        best,
    )
}
