use serde::{Deserialize, Serialize};
use serde_json::{Value}; 
use std::net::UdpSocket;
use std::time::SystemTime;
use std::time::Duration;
use std::{str, fs};

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

#[derive(Serialize, Deserialize, Clone)]
pub struct WiFiDevice{
    ip_addr: String,
    mac: String
}

impl WiZRGBBulb{
    fn get_status(&mut self) -> bool{
        let data = r#"
        {
            "method":"getPilot",
            "params":{}
        }"#;

        let result = self.socket.send_to(data.as_bytes(), &self.ip_addr);
        match result {
            Ok(_status)=>{
                let mut buf = [0; 192]; 
                let (_amt, _src) = self.socket.recv_from(&mut buf).expect("Couldn't get stuff back from socket");
                let res = String::from_utf8(buf.to_vec()).expect("Found invalid UTF-8");
                let res = res.trim_matches(char::from(0));
                
                // Cast all rgbcw values 
                let v: Value = serde_json::from_str(&res).unwrap(); 
                
                self.current_rgbcw.r = v["result"]["r"].to_string().parse::<u8>().unwrap();
                self.current_rgbcw.g = v["result"]["g"].to_string().parse::<u8>().unwrap();
                self.current_rgbcw.b = v["result"]["b"].to_string().parse::<u8>().unwrap();
                self.current_rgbcw.c = v["result"]["c"].to_string().parse::<u8>().unwrap();
                self.current_rgbcw.w = v["result"]["w"].to_string().parse::<u8>().unwrap();
                return true; 
            }, 
            Err(e)=>{
                println!("Couldn't connect to socket: {}", e);
                return false; 
            }
        }
    }

    pub fn set_rgbcw(&mut self, rgbcw: RGBCW) -> bool{
        self.current_rgbcw = rgbcw.clone(); 
        
        let req_struct = WiZBulbRGBCWReq{
            id: 1, 
            method: String::from("setPilot"), 
            params: rgbcw
        };

        let v = serde_json::to_string(&req_struct).unwrap();
        match self.socket.send_to((&v.to_string()).as_bytes(), &self.ip_addr){
            Ok(_a)=>{
                let mut buf = [0; 96];
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

pub fn discover_devices() -> Vec<WiFiDevice>{
    let mut list = Vec::with_capacity(255);
    let data = r#"
    {
        "method":"getPilot",
        "params":{}
    }"#;
    
    // Appending port to socket conenction
    let mut ip_addr_n = String::from("255.255.255.255");
    let self_ip = String::from("192.168.1.37:38899");
    let socket_setup = UdpSocket::bind(&self_ip); 
    ip_addr_n.push_str(":38899");

    let mut mac_vec = Vec::with_capacity(255);
    let mut addr_vec = Vec::with_capacity(255);

    match socket_setup {
        Ok(sock)=>{
            // Set socket timeout
            let _ = sock.set_broadcast(true); 
            let _ = sock.set_read_timeout(Some(Duration::new(5, 0)));
            sock.send_to(&data.as_bytes(), &ip_addr_n).unwrap(); 

            // Buffer and check to keep scanning until we experience a timeout failiure
            let mut buf = [0; 256];
            let mut continue_scanning = true;
            while continue_scanning {
                match sock.recv_from(&mut buf){
                    Ok(a)=>{
                        // Parse out the mac address
                        let json_str = String::from_utf8(buf.to_vec()).unwrap();
                        let json_no_whitespace = json_str.trim_matches(char::from(0)).to_owned().to_string();
                        addr_vec.push(a.1.to_string());
                        mac_vec.push(json_no_whitespace[50..62].to_string());
                    }
                    Err(_e)=>{
                        continue_scanning = false; 
                    }
                }
            }
        }
        Err(e)=>{
            panic!("Error Binding socket for WiZ light: {}", e);
        }
    }   

    // Add to list of discovered devices
    for n in 0..mac_vec.len(){
        println!("Addr: {}, MAC: {}", addr_vec[n], mac_vec[n]);
        let wifi_device = WiFiDevice{
            mac: mac_vec[n].clone(), 
            ip_addr: addr_vec[n].clone()
        };
        list.push(wifi_device);
    }

    return list;
}

pub fn _setup_wiz_lightpub(mut ip_addr_n: String) -> Result<WiZRGBBulb, bool>{
    // Appending port to socket conenction
    let self_ip = String::from("192.168.1.37:38899");
    let socket_setup = UdpSocket::bind(&self_ip); 

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
                
                let mut bulb = WiZRGBBulb{
                    ip_addr: ip_addr_n, 
                    current_rgbcw: led_val, 
                    socket: sock, 
                    last_connection_date: get_unix_time(),
                    last_connection_status: DeviceConnectionStatus::DeviceConnected, 
                    last_light_status: DeviceLightStatus::LightOn
                }; 

                bulb.get_status(); 
                return Ok(bulb);
            }

            return Err(false); 
        }
        Err(e)=>{
            println!("Error Binding socket for WiZ light: {}", e);
            return Err(false);
        }
    }
}

pub fn setup_wiz_light(mut ip_addr_n: String) -> Result<WiZRGBBulb, bool>{
    ip_addr_n.push_str(":38899");
    return _setup_wiz_lightpub(ip_addr_n);
}

pub fn setup_wiz_multicast() -> WiZRGBBulb{
    // Appending port to socket conenction
    let mut ip_addr_n = String::from("255.255.255.255");
    let self_ip = String::from("192.168.1.37:38899");
    let socket_setup = UdpSocket::bind(&self_ip); 
    ip_addr_n.push_str(":38899");

    match socket_setup {
        Ok(sock)=>{
            // Set socket timeout
            let _ = sock.set_broadcast(true); 
            let _ = sock.set_read_timeout(Some(Duration::new(5, 0)));
            let led_val = RGBCW{
                r: 0, 
                g: 0, 
                b: 0, 
                w: 0, 
                c: 0
            };
            
            let bulb = WiZRGBBulb{
                ip_addr: ip_addr_n, 
                current_rgbcw: led_val, 
                socket: sock, 
                last_connection_date: get_unix_time(),
                last_connection_status: DeviceConnectionStatus::DeviceConnected, 
                last_light_status: DeviceLightStatus::LightOn
            }; 

            //bulb.get_status(); 
            return bulb; 
        }
        Err(e)=>{
            panic!("Error Binding socket for WiZ light: {}", e);
        }
    }   
}

fn init_device_directory(file_path: String){
    let filename = file_path; 
    println!("Reading from file: {}", &filename); 
    let contents = fs::read_to_string(&filename).expect("Could not read device file"); 
    let mut init_new_filesystem = false; 

    let mut content: Vec<WiFiDevice> = Vec::with_capacity(255);

    if contents.len() > 0{
        match serde_json::from_str(&contents){
            Ok(a)=>{ 
                content = a; 
            }
            Err(e)=>{
                init_new_filesystem = true; 
            }
        }
    }else{
        init_new_filesystem = true; 
    }

    if init_new_filesystem {
        // Discover new devices on the network. 
        let content = discover_devices();
        let device_list_json = serde_json::to_string(&content).unwrap();

        match fs::remove_file(&filename){
            Ok(a) => {
                println!("Delete file correctly...")
            },
            Err(e)=>{
                println!("Couldn't delete file: {}", e);
            }
        }
    }
    
}
