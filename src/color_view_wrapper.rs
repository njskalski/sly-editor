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

use cursive::Printer;
use cursive::view::View;
use cursive::view::ViewWrapper;
use cursive::theme::Theme;
use std::rc::Rc;

pub type PrinterModifierType = Rc<Box<Fn(&Printer) -> Theme>>;

pub struct ColorViewWrapper<T: View> {
    pub view: T,
    pub printer_modifier : PrinterModifierType
}

impl <T: View> ColorViewWrapper<T> {
    pub fn new(view: T, printer_modifier : PrinterModifierType) -> Self
      where
        T: View/* + ?Sized*/,
        {
            ColorViewWrapper {
                view : view,
                printer_modifier : printer_modifier
            }
        }

    inner_getters!(self.view: T);
}

impl <T: View + Sized> ViewWrapper for ColorViewWrapper<T> {
    wrap_impl!(self.view: T);

    fn wrap_draw(&self, printer: &Printer)
    {
        let new_theme = { (self.printer_modifier)(printer) };
        debug!("new_theme : {:?}", new_theme);
        printer.with_theme(&new_theme, |printer| {
            self.view.draw(printer);
        });
    }
}
