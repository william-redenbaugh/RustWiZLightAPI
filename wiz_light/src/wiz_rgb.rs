use serde::{Deserialize, Serialize};
use std::net::UdpSocket;
use std::time::SystemTime;
use std::time::Duration;
use std::str;

#[derive(Serialize, Deserialize, Clone)]
pub struct RGBCW{
    pub r: u8, 
    pub g: u8, 
    pub b: u8, 
    pub w: u8,
    pub c: u8, 
}

pub enum DeviceConnectionStatus {
    DeviceConnected, 
    DeviceDisconnected, 
    DeviceNotFound
}

pub enum DeviceLightStatus{ 
    LightOff, 
    LightOn
}

#[derive(Serialize, Deserialize)]
struct WiZBulbRGBCWReq{
    id: i32, 
    method: String, 
    params: RGBCW
}

pub struct WiZRGBBulb{
    ip_addr: String, 
    current_rgbcw: RGBCW,
    socket: UdpSocket, 
    last_connection_date: u64,  
    last_connection_status: DeviceConnectionStatus,
    last_light_status: DeviceLightStatus
}

impl WiZRGBBulb{
    pub fn set_rgbcw(&mut self, rgbcw: RGBCW) -> bool{
        self.current_rgbcw = rgbcw.clone(); 
        
        let req_struct = WiZBulbRGBCWReq{
            id: 1, 
            method: String::from("setPilot"), 
            params: rgbcw
        };

        let v = serde_json::to_string(&req_struct).unwrap();
        println!("json: {}", &v);
        match self.socket.send_to((&v.to_string()).as_bytes(), &self.ip_addr){
            Ok(_a)=>{
                let mut buf = [0; 192];
                let (amt, _src) = self.socket.recv_from(&mut buf).unwrap();
                println!("bytearray: {:?}", str::from_utf8(&buf));
                if amt > 0{
                    return true
                }
            }, 
            Err(e)=>{
                println!("Couldn't connect to socket: {}", e);
                return false; 
            }
        }
        self.last_light_status = DeviceLightStatus::LightOn; 
        return false; 
    }

    pub fn turn_on(&mut self) -> bool{
        let data = r#"
        {
            "id":1,
            "method":"setState",
            "params":{"state":true}
        }"#;

        let result = self.socket.send_to(data.as_bytes(), &self.ip_addr);
        match result {
            Ok(_status)=>{
                let mut buf = [0; 192];
                let (amt, _src) = self.socket.recv_from(&mut buf).unwrap();
                if amt > 0{
                    return true
                }
            }, 
            Err(e)=>{
                println!("Couldn't connect to socket: {}", e);
                return false; 
            }
        }

        self.last_light_status = DeviceLightStatus::LightOn; 
        return true; 
    }

    pub fn turn_off(&mut self) -> bool{
        let data = r#"
        {
            "id":1,
            "method":"setState",
            "params":{"state":false}
        }"#;

        let result = self.socket.send_to(data.as_bytes(), &self.ip_addr);
        match result {
            Ok(_status)=>{
                let mut buf = [0; 192];
                let (amt, _src) = self.socket.recv_from(&mut buf).unwrap();
                if amt > 0{
                    return true
                }
            }, 
            Err(e)=>{
                println!("Couldn't connect to socket: {}", e);
                return false; 
            }
        }

        self.last_light_status = DeviceLightStatus::LightOff; 
        return true; 
    }

}

fn handshake(socket: &UdpSocket, target_ip: &String) -> bool{
    let data = r#"
    {
        "id":1,
        "method":"setState",
        "params":{"state":true}
    }"#;

    let result = socket.send_to(data.as_bytes(), target_ip);
    match result {
        Ok(_status)=>{

            let mut buf = [0; 192];
            let (amt, _src) = socket.recv_from(&mut buf).unwrap();

            if amt > 0{
                return true
            }
        }, 
        Err(e)=>{
            println!("IP addr: {}", target_ip);
            println!("Couldn't connect to socket: {}", e);
            return false; 
        }
    }

    return true; 
}

fn get_unix_time() -> u64 {
    let mut m = 0; 
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => m = n.as_secs() as u64,
        Err(_) => panic!("Couldn't get unix timestamp during init process..."),
    }

    return m; 
}

pub fn setup_wiz_light(mut ip_addr_n: String) -> Result<WiZRGBBulb, bool>{
    // Appending port to socket conenction
    let self_ip = String::from("192.168.1.7:38899");
    let socket_setup = UdpSocket::bind(&self_ip); 
    ip_addr_n.push_str(":38899");

    match socket_setup {
        Ok(sock)=>{
            // Set socket timeout
            let result = sock.set_read_timeout(Some(Duration::new(5, 0)));
            
            if handshake(&sock, &ip_addr_n){
                let led_val = RGBCW{
                    r: 0, 
                    g: 0, 
                    b: 0, 
                    w: 0, 
                    c: 0
                };

                return Ok(WiZRGBBulb{
                    ip_addr: ip_addr_n, 
                    current_rgbcw: led_val, 
                    socket: sock, 
                    last_connection_date: get_unix_time(),
                    last_connection_status: DeviceConnectionStatus::DeviceConnected, 
                    last_light_status: DeviceLightStatus::LightOn
                });
            }

            return Err(false); 
        }
        Err(e)=>{
            println!("Error Binding socket for WiZ light: {}", e);
            return Err(false);
        }
    }
}