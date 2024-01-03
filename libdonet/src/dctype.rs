// DONET SOFTWARE
// Copyright (c) 2024, Donet Authors.
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License version 3.
// You should have received a copy of this license along
// with this source code in a file named "LICENSE."
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program; if not, write to the Free Software Foundation,
// Inc., 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.

use crate::datagram::{Datagram, DatagramIterator};
use crate::globals::DgSizeTag;
use crate::hashgen::DCHashGenerator;
use strum_macros::EnumIs;

/* The enum variants defined below have assigned u8 values
 * to keep compatibility with Astron's DC hash inputs.
 */
#[repr(u8)] // 8-bit alignment, unsigned
#[derive(Clone)]
#[rustfmt::skip]
pub enum DCTypedefType {
    // Numeric Types
    TInt8 = 0, TInt16 = 1, TInt32 = 2, TInt64 = 3,
    TUInt8 = 4, TChar = 8, TUInt16 = 5, TUInt32 = 6, TUInt64 = 7,
    TFloat32 = 9, TFloat64 = 10,

    // Sized Data Types (Array Types)
    TString = 11, // a string with a fixed byte length
    TVarString = 12, // a string with a variable byte length
    TBlob = 13, TVarBlob = 14,
    TBlob32 = 19, TVarBlob32 = 20,
    TArray = 15, TVarArray = 16,

    // Complex DC Types
    TStruct = 17, TMethod = 18,
    TInvalid = 21,
}

pub struct DCTypeDefinition {
    alias: Option<String>,
    data_type: DCTypedefType,
    size: DgSizeTag,
}

pub trait DCTypeDefinitionInterface {
    fn new() -> DCTypeDefinition;
    fn generate_hash(&self, hashgen: &mut DCHashGenerator);

    fn get_dc_type(&self) -> DCTypedefType;
    fn is_variable_length(&self) -> bool;
    fn get_size(&self) -> DgSizeTag;

    fn has_alias(&self) -> bool;
    fn get_alias(&self) -> Result<String, ()>;
    fn set_alias(&mut self, alias: String);
}

impl DCTypeDefinitionInterface for DCTypeDefinition {
    fn new() -> DCTypeDefinition {
        DCTypeDefinition {
            alias: None,
            data_type: DCTypedefType::TInvalid,
            size: 0_u16,
        }
    }

    fn generate_hash(&self, hashgen: &mut DCHashGenerator) {
        hashgen.add_int(u32::from(self.data_type.clone() as u8));
        if self.alias.is_some() {
            hashgen.add_string(self.alias.clone().unwrap())
        }
    }

    fn get_dc_type(&self) -> DCTypedefType {
        self.data_type.clone()
    }

    fn is_variable_length(&self) -> bool {
        self.size == 0_u16
    }

    fn get_size(&self) -> DgSizeTag {
        self.size.clone()
    }

    fn has_alias(&self) -> bool {
        self.alias.is_some()
    }

    fn get_alias(&self) -> Result<String, ()> {
        if self.alias.is_some() {
            Ok(self.alias.clone().unwrap())
        } else {
            Err(())
        }
    }

    fn set_alias(&mut self, alias: String) {
        self.alias = Some(alias);
    }
}

// ---------- DC Number ---------- //

#[rustfmt::skip]
#[derive(Clone, EnumIs)]
pub enum DCNumberType {
    None = 0, Int, UInt, Float,
}

#[repr(C)]
#[derive(Copy, Clone)] // required for unwrapping when in an option type
pub union DCNumberValueUnion {
    integer: i64,
    unsigned_integer: u64,
    floating_point: f64,
}

#[derive(Clone)]
struct DCNumber {
    pub number_type: DCNumberType,
    pub value: DCNumberValueUnion,
}

impl DCNumber {
    pub fn new() -> Self {
        Self {
            number_type: DCNumberType::None,
            value: DCNumberValueUnion { integer: 0_i64 },
        }
    }
    pub fn new_integer(num: i64) -> Self {
        Self {
            number_type: DCNumberType::Int,
            value: DCNumberValueUnion { integer: num },
        }
    }
    pub fn new_unsigned_integer(num: u64) -> Self {
        Self {
            number_type: DCNumberType::UInt,
            value: DCNumberValueUnion {
                unsigned_integer: num,
            },
        }
    }
    pub fn new_floating_point(num: f64) -> Self {
        Self {
            number_type: DCNumberType::Float,
            value: DCNumberValueUnion { floating_point: num },
        }
    }
}

// --------- DC Numeric Range --------- //

/* Numeric Range structs are used to represent a range of signed/unsigned
 * integers or floating point numbers. Used for enforcing numeric limits
 * withing constraints of array, string, or blob sized types.
 */
#[derive(Clone)]
struct DCNumericRange {
    range_type: DCNumberType,
    min: DCNumber,
    max: DCNumber,
}

impl DCNumericRange {
    fn new() -> Self {
        let mut default_min: DCNumber = DCNumber::new_floating_point(f64::NEG_INFINITY);
        let mut default_max: DCNumber = DCNumber::new_floating_point(f64::INFINITY);

        default_min.number_type = DCNumberType::None;
        default_max.number_type = DCNumberType::None;

        Self {
            range_type: DCNumberType::None,
            min: default_min,
            max: default_max,
        }
    }

    fn new_integer_range(min: i64, max: i64) -> Self {
        Self {
            range_type: DCNumberType::Int,
            min: DCNumber::new_integer(min),
            max: DCNumber::new_integer(max),
        }
    }

    fn new_unsigned_integer_range(min: u64, max: u64) -> Self {
        Self {
            range_type: DCNumberType::UInt,
            min: DCNumber::new_unsigned_integer(min),
            max: DCNumber::new_unsigned_integer(max),
        }
    }

    fn new_floating_point_range(min: f64, max: f64) -> Self {
        Self {
            range_type: DCNumberType::Float,
            min: DCNumber::new_floating_point(min),
            max: DCNumber::new_floating_point(max),
        }
    }

    fn contains(&self, num: DCNumber) -> bool {
        match self.min.number_type {
            DCNumberType::None => true,
            DCNumberType::Int => unsafe {
                /* NOTE: All reads of unions require an unsafe block due to potential UB.
                 * As the developer, we have to make sure every read of this union guarantees
                 * that it will contain the expected data type at the point we read.
                 * We make use of the DCNumberType enumerator to guarantee this safety.
                 */
                self.min.value.integer <= num.value.integer && num.value.integer <= self.max.value.integer
            },
            DCNumberType::UInt => unsafe {
                self.min.value.unsigned_integer <= num.value.unsigned_integer
                    && num.value.unsigned_integer <= self.max.value.unsigned_integer
            },
            DCNumberType::Float => unsafe {
                self.min.value.floating_point <= num.value.floating_point
                    && num.value.floating_point <= self.max.value.floating_point
            },
        }
    }

    fn is_empty(&self) -> bool {
        self.range_type.is_none() // using strum macro
    }
}

// ---------- Numeric Type ---------- //

struct DCNumericType {
    parent: DCTypeDefinition,
    divisor: u16,
    // These are the original range and modulus values from the file, unscaled by the divisor.
    orig_modulus: f64,
    orig_range: DCNumericRange,
    // These are the range and modulus values after scaling by the divisor.
    modulus: DCNumber,
    range: DCNumericRange,
}

trait DCNumericTypeInterface {
    fn new(base_type: DCTypeDefinition) -> DCNumericType;
    fn generate_hash(&self, hashgen: &mut DCHashGenerator);

    fn has_modulus(&self) -> bool;
    fn has_range(&self) -> bool;

    fn get_divisor(&self) -> u16;
    fn get_modulus(&self) -> f64;
    fn get_range(&self) -> DCNumericRange;

    fn set_divisor(&mut self, divisor: u16) -> Result<(), ()>;
    fn set_modulus(&mut self, modulus: f64) -> Result<(), ()>;
    fn set_range(&mut self, range: DCNumericRange) -> Result<(), ()>;

    fn within_range(&self, data: Vec<u8>, length: u64) -> Result<(), ()>;
}

impl DCNumericType {
    fn data_to_number(&self, data: Vec<u8>) -> (bool, DCNumber) {
        // NOTE: See 'Deref' trait implementation for 'DCNumericType' below
        // on how we're using self.parent.size as self.size.
        if self.size != data.len().try_into().unwrap() {
            return (false, DCNumber::new_integer(0_i64));
        }

        let mut dg = Datagram::new();
        let _ = dg.add_data(data);
        let mut dgi = DatagramIterator::new(dg);

        match self.data_type {
            DCTypedefType::TInt8 => (true, DCNumber::new_integer(i64::from(dgi.read_i8()))),
            DCTypedefType::TInt16 => (true, DCNumber::new_integer(i64::from(dgi.read_i16()))),
            DCTypedefType::TInt32 => (true, DCNumber::new_integer(i64::from(dgi.read_i32()))),
            DCTypedefType::TInt64 => (true, DCNumber::new_integer(dgi.read_i64())),
            DCTypedefType::TChar | DCTypedefType::TUInt8 => {
                (true, DCNumber::new_unsigned_integer(u64::from(dgi.read_u8())))
            }
            DCTypedefType::TUInt16 => (true, DCNumber::new_unsigned_integer(u64::from(dgi.read_u16()))),
            DCTypedefType::TUInt32 => (true, DCNumber::new_unsigned_integer(u64::from(dgi.read_u32()))),
            DCTypedefType::TUInt64 => (true, DCNumber::new_unsigned_integer(dgi.read_u64())),
            DCTypedefType::TFloat32 => (true, DCNumber::new_floating_point(f64::from(dgi.read_f32()))),
            DCTypedefType::TFloat64 => (true, DCNumber::new_floating_point(dgi.read_f64())),
            _ => (false, DCNumber::new_integer(0_i64)),
        }
    }
}

impl DCNumericTypeInterface for DCNumericType {
    fn new(base_type: DCTypeDefinition) -> DCNumericType {
        todo!();
    }

    fn generate_hash(&self, hashgen: &mut DCHashGenerator) {
        self.parent.generate_hash(hashgen);
        hashgen.add_int(u32::from(self.divisor));

        if self.has_modulus() {
            // unsafe block required for accessing unions
            unsafe {
                hashgen.add_int(self.modulus.value.integer.try_into().unwrap());
            }
        }
        if self.has_range() {
            unsafe {
                hashgen.add_int(self.range.min.value.integer.try_into().unwrap());
                hashgen.add_int(self.range.max.value.integer.try_into().unwrap());
            }
        }
    }

    fn has_modulus(&self) -> bool {
        self.orig_modulus != 0.0
    }
    fn has_range(&self) -> bool {
        self.orig_range.is_empty()
    }
    fn get_divisor(&self) -> u16 {
        self.divisor.clone()
    }
    fn get_modulus(&self) -> f64 {
        self.orig_modulus.clone()
    }
    fn get_range(&self) -> DCNumericRange {
        self.orig_range.clone()
    }

    fn set_divisor(&mut self, divisor: u16) -> Result<(), ()> {
        if divisor == 0 {
            return Err(());
        }
        self.divisor = divisor;
        if self.has_range() {
            self.set_range(self.orig_range.clone())?;
        }
        if self.has_modulus() {
            self.set_modulus(self.orig_modulus.clone())?;
        }
        Ok(())
    }

    fn set_modulus(&mut self, modulus: f64) -> Result<(), ()> {
        todo!();
    }
    fn set_range(&mut self, range: DCNumericRange) -> Result<(), ()> {
        todo!();
    }
    fn within_range(&self, data: Vec<u8>, length: u64) -> Result<(), ()> {
        todo!();
    }
}

/* By manually implementing/overriding the standard
 * library's 'Deref' trait of our 'child' struct, we
 * can implicitly cast pointers to the parent struct,
 * as pointers to the child struct, which gives us a
 * nice 'cheat' for the feel of inheritance.
 */
impl std::ops::Deref for DCNumericType {
    type Target = DCTypeDefinition;
    fn deref(&self) -> &Self::Target {
        &self.parent
    }
}

// ---------- Array Type ---------- //

// ---------- Method Type ---------- //