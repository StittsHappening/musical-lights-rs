#![no_std]
#![cfg_attr(not(test), no_main)]
#![feature(type_alias_impl_trait)]

pub mod battery;
pub mod compass;
pub mod config;
pub mod gps;
pub mod hanning;
pub mod lights;
pub mod logging;
pub mod microphone;
pub mod orientation;
pub mod radio;
pub mod sd;
pub mod speaker;
