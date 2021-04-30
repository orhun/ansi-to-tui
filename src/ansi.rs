use crate::error::Error;
use crate::stack::{AnsiGraphicsStack, Stack};
#[cfg(feature = "simd")]
use simdutf8::basic::from_utf8;
use tui::{
    style::Style,
    text::{Span, Spans, Text},
};

/// This functions converts the ascii byte sequence with ansi colors to tui::text::Text type  
/// This functions's argument implements into_iter so the buffer will be consumed on use.
///
/// Example
/// ```rust
/// use ansi_to_tui::ansi_to_text;
/// let bytes : Vec<u8> = vec![b'\x1b', b'[', b'3', b'1', b'm', b'A', b'A', b'A', b'\x1b', b'[', b'0'];
/// let text = ansi_to_text(bytes);
/// ```
///
pub fn ansi_to_text<'t, B: IntoIterator<Item = u8>>(bytes: B) -> Result<Text<'t>, Error> {
    // let reader = bytes.as_ref().iter().copied(); // copies the whole buffer to memory
    let reader = bytes.into_iter();
    // let _read = bytes.as_ref().into_iter();

    let mut buffer: Vec<Spans> = Vec::new();
    let mut line_buffer: Vec<u8> = Vec::new(); // this contains all the text in a single styled ( including utf-8 )
    let mut line_styled_buffer: Vec<u8> = Vec::new(); //this is used to store the text while style is being processed.
    let mut span_buffer: Vec<Span> = Vec::new(); // this contains text with a style and there maybe multiple per line

    let mut style: Style = Style::default();

    let mut stack: Stack<u8> = Stack::new();
    let mut ansi_stack: AnsiGraphicsStack = AnsiGraphicsStack::new();
    let mut style_stack: Stack<Style> = Stack::new();

    style_stack.push(style);

    let mut last_byte = 0_u8;

    for byte in reader {
        // let byte_char = char::from(byte);

        if ansi_stack.is_unlocked() && last_byte == b'\x1b' && byte != b'[' {
            // if byte after \x1b was not [ lock the stack
            ansi_stack.lock();
        }
        // don't use UnicodeWidthChar since we are also parsing utf8.
        // But if there is some error in the byte sequence then
        // if ansi_stack.is_locked() && UnicodeWidthChar::width(byte_char).is_some() {
        if ansi_stack.is_locked() && byte != b'\n' && byte != b'\x1b' {
            // line_buffer.push(byte)

            // Implemented
            // \e[31mHELLO\e[0m\e[31mTEST -> \e[31mHELLOTEST

            // if we find a byte after the new stack has been parsed we do
            // if style_stack.last().unwrap() == &style {}
            if line_styled_buffer.is_empty() {
                // if !line_buffer.is_empty() {
                if style_stack.last().unwrap() != &style && !line_buffer.is_empty() {
                    // } else {
                    span_buffer.push(Span::styled(
                        #[cfg(feature = "simd")]
                        from_utf8(&line_buffer)?.to_owned(),
                        #[cfg(not(feature = "simd"))]
                        String::from_utf8(line_buffer.clone())?,
                        style_stack.pop().unwrap(),
                    ));
                    line_buffer.clear();
                    style_stack.push(style);
                }
            }
            line_styled_buffer.push(byte);
        } else {
            match byte {
                b'\x1b' => {
                    if !line_styled_buffer.is_empty() {
                        line_buffer.append(&mut line_styled_buffer);
                        line_styled_buffer.clear();
                    }
                    ansi_stack.unlock();
                } // this clears the stack

                b'\n' => {
                    // println!("span_buffer {:#?}", span_buffer);
                    // println!("line_buffer {:#?}", line_buffer);
                    // If line buffer is not empty when a newline is detected push the line_buffer
                    // to the span_buffer since we need the spans.
                    // if style_stack.last().unwrap() == &stack {}
                    if !line_styled_buffer.is_empty() {
                        line_buffer.append(&mut line_styled_buffer);
                        line_styled_buffer.clear();
                    }

                    if !line_buffer.is_empty() {
                        span_buffer.push(Span::styled(
                            #[cfg(feature = "simd")]
                            from_utf8(&line_buffer)?.to_owned(),
                            #[cfg(not(feature = "simd"))]
                            String::from_utf8(line_buffer.clone())?,
                            style,
                        ));
                        line_buffer.clear();
                    }

                    if !span_buffer.is_empty() {
                        buffer.push(Spans::from(span_buffer.clone()));
                        span_buffer.clear();
                    } else {
                        buffer.push(Spans::default())
                    }
                    span_buffer.clear();
                }

                b';' => ansi_stack.push(stack.parse_usize()?),

                b'0'..=b'9' => stack.push(byte),

                b'm' => {
                    ansi_stack.push(stack.parse_usize()?);
                    // patch since the last style is not overwritten, only modified with a new
                    // sequence.
                    style = style.patch(ansi_stack.parse_ansi()?);
                    // lock after parse since lock will clear
                    ansi_stack.lock();
                }

                b'[' => (),

                _ => {
                    // any unexpected sequence will cause the ansi graphics stack to lock up
                    ansi_stack.lock();
                }
            }
        }
        last_byte = byte;
    }

    if !line_buffer.is_empty() {
        span_buffer.push(Span::styled(
            #[cfg(feature = "simd")]
            from_utf8(&line_buffer)?.to_owned(),
            #[cfg(not(feature = "simd"))]
            String::from_utf8(line_buffer.clone())?,
            style,
        ));
        line_buffer.clear();
    }
    if !span_buffer.is_empty() {
        buffer.push(Spans::from(span_buffer));
        // span_buffer.clear();
    }

    Ok(buffer.into())
}
