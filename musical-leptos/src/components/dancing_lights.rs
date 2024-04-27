use js_sys::Float64Array;
use leptos::*;
use log::warn;
use musical_lights_core::{
    audio::{
        AggregatedAmplitudesBuilder, AudioBuffer, DownResistanceBuilder, ExponentialScaleBuilder,
        FlatWeighting, PeakScaledBuilder, Samples, FFT,
    },
    lights::Gradient,
    logging::{info, trace},
    windows::HanningWindow,
};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use web_sys::{MediaStream, MediaStreamConstraints, MessageEvent};

use crate::wasm_audio::wasm_audio;

/// TODO: this was 512. i think we probably want that still. but web defaults to 128 so this is simplest
const MIC_SAMPLES: usize = 128;
const FFT_INPUTS: usize = 2048;
/// TODO: i don't like this name
/// 24 to match the Bark Scale
const NUM_CHANNELS: usize = 120;

// const AUDIO_Y: usize = 9;

const FFT_OUTPUTS: usize = FFT_INPUTS / 2;

/// Prompt the user for their microphone
async fn load_media_stream() -> Result<MediaStream, JsValue> {
    let navigator = window().navigator();

    let mut constraints = MediaStreamConstraints::new();
    constraints.audio(&JsValue::from(true));

    let promise = navigator
        .media_devices()
        .unwrap()
        .get_user_media_with_constraints(&constraints)
        .unwrap();

    let f = wasm_bindgen_futures::JsFuture::from(promise);

    let stream: MediaStream = f.await?.unchecked_into();

    Ok(stream)
}

#[component]
pub fn DancingLights() -> impl IntoView {
    // TODO: do this on button click
    let (listen, set_listen) = create_signal(false);

    // TODO: i think this needs to be a vec of signals
    let (audio, set_audio) = create_signal([0.0; NUM_CHANNELS]);

    let (sample_rate, set_sample_rate) = create_signal(None);

    // let gradient = Gradient::<NUM_CHANNELS>::new_mermaid();
    // TODO: use a signal for this so that we can change it real time
    let gradient = Gradient::<NUM_CHANNELS>::new_rainbow(100.0, 70.0);

    let colors: Vec<_> = gradient
        .colors
        .iter()
        .map(|x| format!("#{:02X}{:02X}{:02X}", x.r, x.g, x.b))
        .collect();

    // // TODO: make this a signal so the user can change it?
    // let peak_decay = 0.99;

    // TODO: this is wrong. this runs immediatly, not on first click. why?
    let start_listening = create_resource(listen, move |x| async move {
        if !x {
            return Ok(None);
        }

        let mut peak_scaled_builder = PeakScaledBuilder::new(0.99);

        let mut down_resistance_builder = DownResistanceBuilder::<NUM_CHANNELS>::new(0.005);

        let media_stream = load_media_stream()
            .await
            .map_err(|x| format!("media stream error: {:?}", x))?;

        let media_stream_id = media_stream.id();

        // // TODO: do we need this? does it or something on it need to be spawned?
        // // TODO: how do we tell this to close?
        // let promise = audio_ctx.resume().unwrap();
        // let _ = wasm_bindgen_futures::JsFuture::from(promise).await.unwrap();

        info!("active media stream: {:?}", media_stream_id);

        let (audio_ctx, audio_worklet_node) = wasm_audio(&media_stream)
            .await
            .map_err(|x| format!("audio_ctx error: {:?}", x))?;

        info!("audio context: {:?}", audio_ctx);

        let new_sample_rate = audio_ctx.sample_rate();

        // TODO: is combining signals like this okay?
        set_sample_rate(Some(new_sample_rate));

        // TODO: what weighting?
        let weighting = FlatWeighting;

        let mut audio_buffer = AudioBuffer::<MIC_SAMPLES, FFT_INPUTS>::new();

        let fft = FFT::<FFT_INPUTS, FFT_OUTPUTS>::new_with_window_and_weighting::<
            HanningWindow<FFT_INPUTS>,
            _,
        >(weighting);

        let scale_builder = ExponentialScaleBuilder::<FFT_OUTPUTS, NUM_CHANNELS>::new(
            0.0,
            20_000.0,
            new_sample_rate,
        );
        // let scale_builder = BarkScaleBuilder::new(new_sample_rate);

        // let mut dancing_lights =
        //     DancingLights::<AUDIO_Y, NUM_CHANNELS, { AUDIO_Y * NUM_CHANNELS }>::new(
        //         gradient, peak_decay,
        //     );

        let onmessage_callback = Closure::new(move |x: MessageEvent| {
            // TODO: this seems fragile. how do we be sure of the data type
            let data = x.data();

            let data = Float64Array::new(&data);

            let data = data.to_vec();

            trace!("raw inputs: {:#?}", data);

            let samples: [f64; MIC_SAMPLES] = data.try_into().unwrap();

            // our fft code wants f32, but js gives us f64
            let samples = samples.map(|x| x as f32);

            // TODO: actual audio processing
            // TODO: this will actually be a vec of 120 f32s when we are done
            audio_buffer.push_samples(Samples(samples));

            // TODO: throttle this
            let buffered = audio_buffer.samples();

            let amplitudes = fft.weighted_amplitudes(buffered);

            let mut visual_loudness = scale_builder.build(amplitudes).0 .0;

            // TODO: use log somehow here?

            // peak_scaled_builder pushes the quietest bins to 0 and the loudest to 1
            peak_scaled_builder.scale(&mut visual_loudness);

            down_resistance_builder.update(&mut visual_loudness);

            set_audio(visual_loudness);
        });

        let port = audio_worklet_node.port().unwrap();

        port.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));

        Closure::forget(onmessage_callback);

        Ok::<_, String>(Some(media_stream_id))
    });

    view! {
        // TODO: i think we have an error handler helper elsewhere
        { move || match start_listening() {
            None | Some(Ok(None)) => view! {
                <button
                    on:click= move |_| {
                        set_listen(true)
                    }
                >
                    Start Dancing (Microphone Access Required)
                </button>
            }.into_view(),
            Some(Ok(Some(media_stream_id))) => view! {
                // <button
                //     on:click= move |_| {
                //         // set_listen(false)
                //         info!("todo: figure out how to turn off the media stream");
                //     }
                // >
                //     Now Listening
                // </button>

                <div id="dancinglights">
                    // TODO: change audio to be a vec of signals and then use a For
                    // <For
                    //     each={move || audio.get().into_iter().enumerate()}
                    //     key=|(i, _val)| *i
                    //     let:data
                    // >
                    //     <li>{data.1}</li>
                    // </For>
                    {audio().into_iter().enumerate().map(|(i, x)| audio_list_item(&colors[i], (x * 8.0) as u8)).collect_view()}
                </div>

                <p>Input ID: { media_stream_id }</p>

                <p>Sample Rate: { sample_rate }Hz</p>
            }.into_view(),
            Some(Err(err)) => view! { <div>Error: {err}</div> }.into_view(),
        }}

    }
}

/// TODO: i think this should be a component, but references make that unhappy
pub fn audio_list_item(color: &str, x: u8) -> impl IntoView {
    let text = match x {
        0 => "󠀠",
        1 => "M",
        2 => "ME",
        3 => "MER",
        4 => "MERB",
        5 => "MERBO",
        6 => "MERBOT",
        7 => "MERBOTS ",
        8 => "MERBOTS!",
        _ => {
            // TODO: we used to have the index here. i think we want that back
            warn!("unexpected length for {}! {}", color, x);
            "ERROR!!!!"
        }
    };

    // TODO: show the frequency on hover
    view! {
        <div style={format!("background-color: {}; color: white;", color)}>{text}</div>
    }
}
