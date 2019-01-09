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

use cursive::theme;
use cursive::theme::Theme;
use cursive::vec::Vec2;
use cursive::view::View;
use cursive::view::ViewWrapper;
use cursive::Printer;
use enumset::EnumSet;
use std::rc::Rc;

pub type PrinterModifierType = Rc<Box<Fn(&Printer) -> Theme>>;

// TODO(njskalski) it has not been decided yet what is the final structure of this wrapper.
pub struct ColorViewWrapper<T : View> {
    view :             T,
    printer_modifier : PrinterModifierType,
    // effects : EnumSet<theme::Effect>,
    // fill_background : bool,
    size : Vec2,
}

impl<T : View> ColorViewWrapper<T> {
    inner_getters!(self.view: T);

    // TODO(njskalski) extend constructor, un-hardcode parameters
    pub fn new(view : T, printer_modifier : PrinterModifierType) -> Self
        where T : View /* + ?Sized */
    {
        ColorViewWrapper { view :             view,
                           printer_modifier : printer_modifier,
                           // effects : EnumSet::new(),
                           // fill_background : true,
                           size : Vec2::new(0, 0), }
    }
}

impl<T : View + Sized> ViewWrapper for ColorViewWrapper<T> {
    wrap_impl!(self.view: T);

    fn wrap_layout(&mut self, size : Vec2) {
        self.size = size;
        self.view.layout(size);
    }

    fn wrap_draw(&self, printer : &Printer) {
        let new_theme = { (self.printer_modifier)(printer) };
        // debug!("new_theme : {:?}", new_theme);

        printer.with_theme(&new_theme, |printer| {
                   // if self.fill_background {
                   //     for y in 0..self.size.y {
                   //         for x in 0..self.size.x {
                   //             printer.print((x, y), " ");
                   //         }
                   //     }
                   // }

                   // printer.with_effects(self.effects, |printer| {
                   self.view.draw(printer);
                   // });
               });
    }
}
