//! Version-specific 1.21.1 inventory packet codecs.
//!
//! These packet payloads must use the protocol-767 item component registry.
//! They deliberately do not use the active 26.3 Slot codec.

use crate::{
    Error, Result,
    buf::{Codec, Reader, Writer},
    item::{DelimitedSlot1211, Hashed, HashedSlot, Slot1211, Stack},
};

use crate::packet_ids_1_21_1::play::{clientbound, serverbound};

/// ClientboundContainerSetContentPacket (1.21.1).
#[derive(Clone, Debug, PartialEq)]
pub struct ContainerSetContent {
    pub container: i32,
    pub state: i32,
    pub items: Vec<Option<Stack>>,
    pub carried: Option<Stack>,
}

impl Codec<ContainerSetContent> for ContainerSetContent {
    fn read(r: &mut Reader) -> Result<ContainerSetContent> {
        let container = r.varint()?;
        let state = r.varint()?;
        let count = r.count(65536)?;
        let mut items = Vec::with_capacity(count.min(128));
        for _ in 0..count {
            items.push(Slot1211::read(r)?);
        }
        let carried = Slot1211::read(r)?;
        Ok(ContainerSetContent {
            container,
            state,
            items,
            carried,
        })
    }

    fn write(value: &ContainerSetContent, w: &mut Writer) {
        w.varint(value.container);
        w.varint(value.state);
        w.varint(value.items.len() as i32);
        for item in &value.items {
            Slot1211::write(item, w);
        }
        Slot1211::write(&value.carried, w);
    }
}

/// ClientboundContainerSetSlotPacket (1.21.1).
#[derive(Clone, Debug, PartialEq)]
pub struct ContainerSetSlot {
    pub container: i32,
    pub state: i32,
    pub slot: i16,
    pub item: Option<Stack>,
}

impl Codec<ContainerSetSlot> for ContainerSetSlot {
    fn read(r: &mut Reader) -> Result<ContainerSetSlot> {
        Ok(ContainerSetSlot {
            container: r.varint()?,
            state: r.varint()?,
            slot: r.i16()?,
            item: Slot1211::read(r)?,
        })
    }

    fn write(value: &ContainerSetSlot, w: &mut Writer) {
        w.varint(value.container);
        w.varint(value.state);
        w.i16(value.slot);
        Slot1211::write(&value.item, w);
    }
}

/// ClientboundSetEquipmentPacket (1.21.1).
#[derive(Clone, Debug, PartialEq)]
pub struct SetEquipment {
    pub entity_id: i32,
    pub slots: Vec<(u8, Option<Stack>)>,
}

impl Codec<SetEquipment> for SetEquipment {
    fn read(r: &mut Reader) -> Result<SetEquipment> {
        let entity_id = r.varint()?;
        let mut slots = Vec::new();
        loop {
            let encoded_slot = r.u8()?;
            slots.push((encoded_slot & 0x7f, Slot1211::read(r)?));
            if slots.len() > 8 {
                return Err(Error::new("too many equipment entries in protocol-767 packet"));
            }
            if encoded_slot & 0x80 == 0 {
                break;
            }
        }
        Ok(SetEquipment { entity_id, slots })
    }

    fn write(value: &SetEquipment, w: &mut Writer) {
        w.varint(value.entity_id);
        for (index, (slot, item)) in value.slots.iter().enumerate() {
            let more = index + 1 < value.slots.len();
            w.u8(if more { *slot | 0x80 } else { *slot });
            Slot1211::write(item, w);
        }
    }
}

/// Changed-slot array in ServerboundContainerClickPacket.
#[derive(Clone, Debug, PartialEq)]
pub struct ChangedSlot {
    pub slot: i16,
    pub item: Option<Hashed>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContainerClick {
    pub container: i32,
    pub state: i32,
    pub slot: i16,
    pub button: i8,
    pub mode: i32,
    pub changed: Vec<ChangedSlot>,
    pub carried: Option<Hashed>,
}

impl Codec<ContainerClick> for ContainerClick {
    fn read(r: &mut Reader) -> Result<ContainerClick> {
        let container = r.varint()?;
        let state = r.varint()?;
        let slot = r.i16()?;
        let button = r.i8()?;
        let mode = r.varint()?;
        let count = r.count(128)?;
        let mut changed = Vec::with_capacity(count);
        for _ in 0..count {
            changed.push(ChangedSlot {
                slot: r.i16()?,
                item: HashedSlot::read(r)?,
            });
        }
        let carried = HashedSlot::read(r)?;
        Ok(ContainerClick {
            container,
            state,
            slot,
            button,
            mode,
            changed,
            carried,
        })
    }

    fn write(value: &ContainerClick, w: &mut Writer) {
        w.varint(value.container);
        w.varint(value.state);
        w.i16(value.slot);
        w.i8(value.button);
        w.varint(value.mode);
        w.varint(value.changed.len() as i32);
        for change in &value.changed {
            w.i16(change.slot);
            HashedSlot::write(&change.item, w);
        }
        HashedSlot::write(&value.carried, w);
    }
}

/// ServerboundSetCreativeModeSlotPacket (1.21.1).
#[derive(Clone, Debug, PartialEq)]
pub struct SetCreativeModeSlot {
    pub slot: i16,
    pub item: Option<Stack>,
}

impl Codec<SetCreativeModeSlot> for SetCreativeModeSlot {
    fn read(r: &mut Reader) -> Result<SetCreativeModeSlot> {
        Ok(SetCreativeModeSlot {
            slot: r.i16()?,
            item: DelimitedSlot1211::read(r)?,
        })
    }

    fn write(value: &SetCreativeModeSlot, w: &mut Writer) {
        w.i16(value.slot);
        DelimitedSlot1211::write(&value.item, w);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Clientbound {
    SetContent(ContainerSetContent),
    SetSlot(ContainerSetSlot),
    Equipment(SetEquipment),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Serverbound {
    Click(ContainerClick),
    CreativeSlot(SetCreativeModeSlot),
}

/// Decode inventory packets whose item payloads use the 1.21.1 component IDs.
pub fn decode_clientbound(id: i32, payload: &[u8]) -> Result<Clientbound> {
    match id {
        clientbound::CONTAINER_SET_CONTENT => {
            decode_exact(payload, ContainerSetContent::read).map(Clientbound::SetContent)
        }
        clientbound::CONTAINER_SET_SLOT => {
            decode_exact(payload, ContainerSetSlot::read).map(Clientbound::SetSlot)
        }
        clientbound::SET_EQUIPMENT => {
            decode_exact(payload, SetEquipment::read).map(Clientbound::Equipment)
        }
        other => Err(Error::new(format!(
            "unsupported protocol-767 inventory clientbound packet {other}"
        ))),
    }
}

/// Encode inventory packets with protocol-767 packet IDs and slot codecs.
pub fn encode_serverbound(packet: &Serverbound) -> Result<Vec<u8>> {
    let mut payload = Writer::new();
    let id = match packet {
        Serverbound::Click(value) => {
            ContainerClick::write(value, &mut payload);
            serverbound::CONTAINER_CLICK
        }
        Serverbound::CreativeSlot(value) => {
            SetCreativeModeSlot::write(value, &mut payload);
            serverbound::SET_CREATIVE_MODE_SLOT
        }
    };
    let mut output = Writer::new();
    output.varint(id);
    output.bytes(&payload.data);
    Ok(output.data)
}

fn decode_exact<T>(payload: &[u8], decode: impl FnOnce(&mut Reader) -> Result<T>) -> Result<T> {
    let mut reader = Reader::new(payload);
    let value = decode(&mut reader)?;
    if reader.remaining() != 0 {
        return Err(Error::new(format!(
            "protocol-767 inventory packet has {} trailing bytes",
            reader.remaining()
        )));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::Component;

    fn round_trip<T: Codec<T> + PartialEq + std::fmt::Debug>(value: T) {
        let mut writer = Writer::new();
        T::write(&value, &mut writer);
        let mut reader = Reader::new(&writer.data);
        assert_eq!(T::read(&mut reader).unwrap(), value);
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn container_content_uses_protocol_767_item_components() {
        let mut stack = Stack::new(42, 3);
        stack.added.push((1, Component::Int(32)));
        round_trip(ContainerSetContent {
            container: 0,
            state: 7,
            items: vec![Some(stack.clone()), None],
            carried: Some(stack),
        });
    }

    #[test]
    fn container_slot_and_equipment_round_trip() {
        let mut stack = Stack::new(5, 1);
        stack.added.push((18, Component::Bool(true)));
        round_trip(ContainerSetSlot {
            container: 2,
            state: 9,
            slot: 4,
            item: Some(stack.clone()),
        });
        round_trip(SetEquipment {
            entity_id: 123,
            slots: vec![(0, Some(stack.clone())), (5, None)],
        });
    }

    #[test]
    fn serverbound_click_and_creative_slot_use_target_ids() {
        let click = ContainerClick {
            container: 1,
            state: 2,
            slot: 4,
            button: 0,
            mode: 0,
            changed: vec![ChangedSlot {
                slot: 4,
                item: Some(Hashed {
                    item: 5,
                    count: 1,
                    added: vec![(1, 0x12345678)],
                    removed: vec![],
                }),
            }],
            carried: None,
        };
        let encoded = encode_serverbound(&Serverbound::Click(click.clone())).unwrap();
        assert_eq!(encoded[0], serverbound::CONTAINER_CLICK as u8);
        assert_eq!(
            decode_exact(&encoded[1..], ContainerClick::read).unwrap(),
            click
        );

        let mut stack = Stack::new(5, 1);
        stack.added.push((1, Component::Int(16)));
        let creative = SetCreativeModeSlot {
            slot: 36,
            item: Some(stack),
        };
        let encoded = encode_serverbound(&Serverbound::CreativeSlot(creative.clone())).unwrap();
        assert_eq!(
            encoded[0],
            serverbound::SET_CREATIVE_MODE_SLOT as u8
        );
        assert_eq!(
            decode_exact(&encoded[1..], SetCreativeModeSlot::read).unwrap(),
            creative
        );
    }

    #[test]
    fn inventory_decoders_reject_trailing_data() {
        let mut writer = Writer::new();
        ContainerSetSlot::write(
            &ContainerSetSlot {
                container: 0,
                state: 1,
                slot: 2,
                item: None,
            },
            &mut writer,
        );
        writer.u8(0);
        assert!(decode_clientbound(
            clientbound::CONTAINER_SET_SLOT,
            &writer.data
        )
        .is_err());
    }
}
