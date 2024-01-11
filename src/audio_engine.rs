use std::sync::mpsc::Receiver;

use cpal::{
    traits::{DeviceTrait, HostTrait},
    Stream,
};

pub fn start(source: Receiver<f32>) -> Stream {
    let host = cpal::host_from_id(
        cpal::available_hosts()
            .into_iter()
            .find(|id| *id == cpal::HostId::Jack)
            .expect("make sure --features jack is specified"),
    )
    .expect("jack host unavailable");

    let device = host.default_output_device().unwrap();

    let config = device.default_output_config().unwrap();

    let err_fn = |err| eprintln!("an error occurred on input stream: {err}");

    let out_stream = device.build_output_stream(
        &config.config(),
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            for (_, frame) in data.chunks_mut(2).enumerate() {
                let Ok(sample) = source.recv() else {
                    println!("Some receiving error at the audio engine side");
                    continue;
                };
                for ch in frame {
                    // TODO: load pipeline defined audio samples
                    *ch = sample;
                }
            }
        },
        err_fn,
        None,
    );

    out_stream.unwrap()
}
