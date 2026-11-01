//! Version-specific 1.21.1 inventory packet codecs.
//!
//! These packet payloads must use the protocol-767 item component registry.
//! They deliberately do not use the active 26.3 Slot codec.

use crate::{
    Error, Result,
    buf::{Codec, Reader, Writer},
    item::{Component, DelimitedSlot1211, Hashed, HashedSlot, Slot1211, Stack},
    packets::play as play_packets,
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
    MerchantOffers(play_packets::MerchantOffers),
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
        clientbound::MERCHANT_OFFERS => {
            decode_merchant_offers(payload).map(Clientbound::MerchantOffers)
        }
        other => Err(Error::new(format!(
            "unsupported protocol-767 inventory clientbound packet {other}"
        ))),
    }
}

/// Decode the 1.21.1 MerchantOffers packet. ItemCost component predicates
/// use target component IDs, while result stacks use the target Slot codec.
fn decode_merchant_offers(payload: &[u8]) -> Result<play_packets::MerchantOffers> {
    fn cost(reader: &mut Reader) -> Result<play_packets::NetCost> {
        let item = reader.varint()?;
        let count = reader.varint()?;
        let components_count = reader.count(256)?;
        let mut components = Vec::with_capacity(components_count.min(64));
        for _ in 0..components_count {
            let kind = reader.varint()?;
            if crate::packets::v1_21_1::component_ids::name(kind).is_none() {
                return Err(Error::new(format!(
                    "unknown protocol-767 merchant item component id {kind}"
                )));
            }
            components.push((kind, Component::read_1_21_1(kind, reader)?));
        }
        Ok(play_packets::NetCost {
            item,
            count,
            components,
        })
    }

    let mut reader = Reader::new(payload);
    let container = reader.varint()?;
    let count = reader.count(4096)?;
    let mut offers = Vec::with_capacity(count.min(64));
    for _ in 0..count {
        let cost_a = cost(&mut reader)?;
        let result = Slot1211::read(&mut reader)?
            .ok_or_else(|| Error::new("empty merchant result in protocol-767 packet"))?;
        let cost_b = if reader.bool()? {
            Some(cost(&mut reader)?)
        } else {
            None
        };
        offers.push(play_packets::NetOffer {
            cost_a,
            result,
            cost_b,
            out_of_stock: reader.bool()?,
            uses: reader.i32()?,
            max_uses: reader.i32()?,
            xp: reader.i32()?,
            special_price: reader.i32()?,
            price_multiplier: reader.f32()?,
            demand: reader.i32()?,
        });
    }
    let packet = play_packets::MerchantOffers {
        container,
        offers,
        level: reader.varint()?,
        xp: reader.varint()?,
        show_progress: reader.bool()?,
        can_restock: reader.bool()?,
    };
    if reader.remaining() != 0 {
        return Err(Error::new(format!(
            "protocol-767 merchant offers packet has {} trailing bytes",
            reader.remaining()
        )));
    }
    Ok(packet)
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
    fn merchant_offers_decode_with_protocol_767_component_ids_and_slots() {
        let mut writer = Writer::new();
        writer.varint(4);
        writer.varint(1);
        writer.varint(5);
        writer.varint(3);
        writer.varint(1);
        writer.varint(1);
        writer.varint(9);
        Slot1211::write(&Some(Stack::new(6, 2)), &mut writer);
        writer.bool(false);
        writer.bool(false);
        writer.i32(0);
        writer.i32(12);
        writer.i32(1);
        writer.i32(0);
        writer.f32(0.05);
        writer.i32(0);
        writer.varint(2);
        writer.varint(17);
        writer.bool(true);
        writer.bool(false);

        let packet = decode_merchant_offers(&writer.data).unwrap();
        assert_eq!(packet.container, 4);
        assert_eq!(packet.level, 2);
        assert_eq!(packet.xp, 17);
        assert!(packet.show_progress);
        assert!(!packet.can_restock);
        assert_eq!(
            packet.offers[0].cost_a.components,
            vec![(1, Component::Int(9))]
        );
        assert_eq!(packet.offers[0].result, Stack::new(6, 2));

        writer.u8(0);
        assert!(decode_merchant_offers(&writer.data).is_err());
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
