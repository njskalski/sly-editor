/*
Copyright 2018 Google LLC

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    https://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

use crate::sly_view::SlyView;
use std::error::Error;
use std::fmt::Display;

/// This trait will represent floating "windows" of interface.
pub trait OverlayDialog<R, E>: SlyView
where
    E: Error + Display,
{
    fn is_displayed(&self) -> bool;
    fn is_finished(&self) -> bool;
    fn get_result(&self) -> Option<Result<R, E>>;
    fn cancel(&mut self);
}
