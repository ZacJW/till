use core::pin::Pin;

trait IOSubsystem {
    type Subscriber: IOSubsystem;

    fn poll(&mut self);
}

trait IOSubscriber {
    type Subsystem: IOSubsystem;

    fn register(self: Pin<&mut Self>, subsystem: &Self::Subsystem);
}

trait SubsystemGroup {
    fn poll(&mut self);
}

trait ProvidesSubsystem<Subsystem: IOSubsystem> {
    fn get(&self) -> &Subsystem;
    fn get_mut(&mut self) -> &mut Subsystem;
}

macro_rules! subsystem_group {
    {
        $(#[$attrs:meta])*
        $vis:vis struct $name:ident {
            $(
                $(#[field_attr:meta])*
                $field_vis:vis $field_name:ident : $field_type:ty,
            )+
        }
    } => {
        $(#[$attrs])*
        $vis struct $name {
            $(
                $(#[field_attr])*
                $field_vis $field_name : $field_type,
            )+
        }

        impl SubsystemGroup for $name {
            fn poll(&mut self) {
                $(
                    <$field_type as IOSubsystem>::poll(&mut self.$field_name);
                )+
            }
        }

        $(
            impl ProvidesSubsystem<$field_type> for $name {
                fn get(&self) -> &$field_type {
                    &self.$field_name
                }

                fn get_mut(&mut self) -> &mut $field_type {
                    &mut self.$field_name
                }
            }
        )+
    };
}
