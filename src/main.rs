extern crate websocket;
extern crate unix_socket;
extern crate openssl;
extern crate regex;
#[macro_use]
extern crate lazy_static;

mod connector;
mod packet;



fn main() {

    //main connection loop
    loop {
        
        //open client
        let mut client = match connector::connect_gdax() {
            Ok(x) => x,
            Err(e) => panic!("Could not open client!\n {:?}", e)
        };

        //loop over messages
        for result_msg in client.incoming_dataframes() {
            
            //get the dataframe
            let df = match result_msg {
                Ok(x) => x,
                Err(ref e ) => panic!("DataFrame Error!\n{:?}",e)
            };

            //get string
            let msg = match connector::df2txt(&df) {
                Ok(x) => x,
                Err(_) => break
            };
        }
    }
}
