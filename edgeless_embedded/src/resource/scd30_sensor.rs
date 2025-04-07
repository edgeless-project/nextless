// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
#[derive(Debug)]
pub struct Measurement {
    pub co2: f32,
    pub rh: f32,
    pub temp: f32,
}

pub struct SCD30SensorInner {
    pub instance_id: Option<edgeless_api_core::instance_id::InstanceId>,
    pub data_out_id: Option<edgeless_api_core::common::Output>,
    pub data_receiver: Option<embassy_sync::channel::Receiver<'static, embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, Measurement, 2>>,
    // pub delay: u8,
}

pub trait Sensor {
    fn init(&mut self, delay_s: u8);
    fn read(&mut self) -> Result<Measurement, ()>;
}

pub struct SCD30SensorConfiguration {
    pub instance_id: edgeless_api_core::instance_id::InstanceId,
    pub data_out_id: Option<edgeless_api_core::common::Output>,
}

pub struct SCD30Sensor {
    pub inner: &'static embassy_sync::mutex::Mutex<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, SCD30SensorInner>,
}

impl SCD30Sensor {
    async fn parse_configuration<'a>(
        data: edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification<'a>,
    ) -> Result<SCD30SensorConfiguration, edgeless_api_core::common::ErrorResponse> {
        let mut out_id: Option<edgeless_api_core::common::Output> = None;

        if data.class_type != "scd30-sensor" {
            return Err(edgeless_api_core::common::ErrorResponse {
                summary: "Wrong Resource ProviderId",
                detail: None,
            });
        }

        for (key, val) in data.output_mapping {
            if key == "data_out" {
                out_id = Some(val);
                break;
            }
        }

        Ok(SCD30SensorConfiguration {
            data_out_id: out_id,
            instance_id: data.instance_id,
        })
    }

    pub async fn new(
        data_receiver: embassy_sync::channel::Receiver<'static, embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, Measurement, 2>,
    ) -> &'static mut dyn crate::resource::ResourceDyn {
        static SENSOR_STATE_RAW: static_cell::StaticCell<
            embassy_sync::mutex::Mutex<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, SCD30SensorInner>,
        > = static_cell::StaticCell::new();
        let sensor_state = SENSOR_STATE_RAW.init_with(|| {
            embassy_sync::mutex::Mutex::new(SCD30SensorInner {
                instance_id: None,
                data_out_id: None,
                data_receiver: Some(data_receiver),
            })
        });
        static SLF_RAW: static_cell::StaticCell<SCD30Sensor> = static_cell::StaticCell::new();
        SLF_RAW.init_with(|| SCD30Sensor { inner: sensor_state })
    }
}

#[embassy_executor::task]
pub async fn scd30_reader_task(
    sensor: &'static mut dyn Sensor,
    sender: embassy_sync::channel::Sender<'static, embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, Measurement, 2>,
) {
    sensor.init(10);
    embassy_time::Timer::after(embassy_time::Duration::from_secs(10_u64)).await;
    loop {
        let data = {
            match sensor.read() {
                Ok(val) => {
                    if !val.co2.is_nan() && !val.rh.is_nan() && !val.rh.is_nan() {
                        // log::info!("{:?}", val);
                        val
                    } else {
                        continue;
                    }
                }
                Err(_) => {
                    continue;
                }
            }
        };
        sender.send(data).await;
        embassy_time::Timer::after(embassy_time::Duration::from_secs(10_u64)).await;
    }
}

impl crate::resource::Resource for SCD30Sensor {
    fn provider_id(&self) -> &'static str {
        "scd30-sensor-bridge-1"
    }

    fn resource_class(&self) -> &'static str {
        "scd30-sensor"
    }

    fn outputs(&self) -> &'static [&'static str] {
        &["data_out"]
    }

    async fn has_instance(&self, instance_id: &edgeless_api_core::instance_id::InstanceId) -> bool {
        let lck = self.inner.lock().await;

        lck.instance_id == Some(*instance_id)
    }

    async fn launch(&mut self, spawner: embassy_executor::Spawner, agent: crate::agent::EmbeddedAgent) {
        spawner.spawn(scd30_sensor_task(self.inner, agent)).unwrap();
    }
}

#[embassy_executor::task]
pub async fn scd30_sensor_task(
    state: &'static embassy_sync::mutex::Mutex<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, SCD30SensorInner>,
    agent: crate::agent::EmbeddedAgent,
) {
    let receiver = {
        let mut lck = state.lock().await;
        lck.data_receiver.take().unwrap()
    };

    loop {
        let measurement = receiver.receive().await;

        let lck = state.lock().await;

        if let (Some(instance_id), Some(data_out_id)) = (lck.instance_id, lck.data_out_id.clone()) {
            let mut dataplane_handle = crate::dataplane::EmbeddedDataplaneHandle::new(instance_id, agent.clone(), heapless::Vec::new());

            let mut buffer = heapless::String::<150>::new();
            if core::fmt::write(
                &mut buffer,
                format_args!("{:.5};{:.5};{:.5}", measurement.co2, measurement.rh, measurement.temp),
            )
            .is_ok()
            {
                match data_out_id {
                    edgeless_api_core::common::Output::Single(id) => {
                        dataplane_handle
                            .send(instance_id, id.instance_id, id.port_id, buffer.as_bytes())
                            .await
                            .unwrap();
                    }
                    edgeless_api_core::common::Output::Any(ids) => {
                        let id = ids.0.first();
                        if let Some(id) = id {
                            dataplane_handle
                                .send(instance_id, id.instance_id, id.port_id.clone(), buffer.as_bytes())
                                .await
                                .unwrap();
                        } else {
                            // return Err(GuestAPIError::UnknownAlias)
                        }
                    }
                    edgeless_api_core::common::Output::All(ids) => {
                        for id in ids.0 {
                            dataplane_handle
                                .send(instance_id, id.instance_id, id.port_id, buffer.as_bytes())
                                .await
                                .unwrap();
                        }
                    }
                }
            }
        }
    }
}

impl crate::invocation::InvocationAPI for SCD30Sensor {
    async fn handle(&mut self, _event: edgeless_api_core::invocation::Event) -> Result<edgeless_api_core::invocation::LinkProcessingResult, ()> {
        log::warn!("SCD30 Sensor received unexpected Event.");
        Ok(edgeless_api_core::invocation::LinkProcessingResult::FINAL)
    }
}

impl crate::resource_configuration::ResourceConfigurationAPI for SCD30Sensor {
    async fn start<'a>(
        &mut self,
        instance_specification: edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification<'a>,
    ) -> Result<(), edgeless_api_core::common::ErrorResponse> {
        let instance_specification = SCD30Sensor::parse_configuration(instance_specification).await?;
        let mut lck = self.inner.lock().await;

        if lck.instance_id.is_some() {
            return Err(edgeless_api_core::common::ErrorResponse {
                summary: "Resource Busy",
                detail: None,
            });
        }

        lck.instance_id = Some(instance_specification.instance_id);
        lck.data_out_id = instance_specification.data_out_id;
        log::info!("Start Sensor");
        Ok(())
    }

    async fn stop(&mut self, resource_id: edgeless_api_core::instance_id::InstanceId) -> Result<(), edgeless_api_core::common::ErrorResponse> {
        let mut lck = self.inner.lock().await;

        if let Some(instance_id) = lck.instance_id {
            if instance_id == resource_id {
                lck.instance_id = None;
                lck.data_out_id = None;
            }
        } else {
            return Err(edgeless_api_core::common::ErrorResponse {
                summary: "Wrong Resource InstanceId",
                detail: None,
            });
        }

        Ok(())
    }

    async fn patch(
        &mut self,
        patch_req: edgeless_api_core::resource_configuration::EncodedPatchRequest<'_>,
    ) -> Result<(), edgeless_api_core::common::ErrorResponse> {
        let mut lck = self.inner.lock().await;

        for (output_key, output_val) in patch_req.output_mapping {
            if output_key == "data_out" {
                lck.data_out_id = Some(output_val);
                break;
            }
        }
        Ok(())
    }
}
