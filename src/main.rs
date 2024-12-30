use std::collections::HashMap;

const NUM_DIRECT_POINTERS: usize = 4;

//define enums/inode types
#[allow(dead_code)]
#[derive(Debug, Clone)]
enum InodeType {
    File,
    Directory,
}
//inode struct 
#[allow(dead_code)]
#[derive(Debug, Clone)]
struct Inode {
    id: u64,
    file_name: String,
    inode_type: InodeType,
    file_size: u64,
    direct_pointers: [Option<u64>;NUM_DIRECT_POINTERS],
    entries: Option<Vec<u64>>,
}
//inode functions
impl Inode {
    //create a new inode 
    fn new_inode(name: &str, inode_type: InodeType, id: u64) -> Self {
        Inode {
            id,
            file_name: name.to_string(),
            inode_type: inode_type.clone(),
            file_size: 0,
            direct_pointers: [None; NUM_DIRECT_POINTERS],
            entries: match inode_type {
                InodeType::Directory => Some(Vec::new()),
                InodeType::File => None,
            },
        } 
    }
    //add an inode 
    fn add_entry(&mut self, child_id: u64){
        if let Some(entries) = &mut self.entries {
            entries.push(child_id);
        }
    }
}

//file system struct 
#[allow(dead_code)]
#[derive(Clone, Debug)]
struct FileSystem {
    inodes: HashMap<u64, Inode>,
    blocks: HashMap<u64, Vec<u8>>,
    id_counter: u64,
    journal: Journal,
}
//file system function 
impl FileSystem {
    //new file system 
    fn new() -> Self {
        FileSystem {
            inodes: HashMap::new(),
            blocks: HashMap::new(),
            id_counter: 0,
            journal: Journal::new(),
        }
    }

    //ccreate a new directory 
    fn create_directory(&mut self, name: &str) -> Inode {
        self.id_counter += 1;
        let new_inode = Inode::new_inode(name, InodeType::Directory, self.id_counter);
        self.inodes.insert(new_inode.id, new_inode.clone());
        println!("A directory named {:?} is created and added to the inode table.", name);

        self.journal.add_entry(format!("CREATE DIRECTORY: {}", name), format!("ID: {}", new_inode.id),);
        self.journal.commit();

        new_inode


    }
    //create a new file 
    fn create_file(&mut self, name: &str) -> Inode{
        self.id_counter += 1;
        let new_inode = Inode::new_inode(name, InodeType::File, self.id_counter);
        self.inodes.insert(new_inode.id, new_inode.clone());
        println!("A file named {:?} is created with an inode assigned.", name);

        self.journal.add_entry(format!("CREATE FILE: {}", name), format!("ID: {}", new_inode.id),);
        self.journal.commit();
        new_inode
    }

    //write to files 
    fn write_to_file(&mut self, mut inode: Inode, data: &[u8]) -> Inode {
        let new_block = self.blocks.len() as u64 + 1; 
        self.blocks.insert(new_block, data.to_vec());
        for i in 0..NUM_DIRECT_POINTERS {
            if inode.direct_pointers[i].is_none() {
                inode.direct_pointers[i] = Some(new_block);
                break;
            }
        }
        inode.file_size += data.len() as u64;
        println!("Data is written to the file {:?} using direct ponters.", inode.file_name);
        inode
    }

    //add file to a directory 
    fn add_file_to_directory(&mut self, directory_inode: &mut Inode, file_inode: &Inode) {
        if matches!(directory_inode.inode_type, InodeType::Directory) {
            directory_inode.add_entry(file_inode.id);
            println!("The file {:?} is added to the {:?} directory.", file_inode.file_name, directory_inode.file_name);

            self.journal.add_entry(format!("ADD FILE: {} TO DIRECTORY: {}", file_inode.id, directory_inode.id), format!("ID: {}, Parent ID: {}", file_inode.id, directory_inode.id),);
            self.journal.commit();
        } else {
            println!("Error: Provided Inode is not a directory!");
        }

    }

    //read a file 
    fn read_file(&mut self, file: &Inode) -> String {
        let mut file_data = Vec::new();

        for i in 0..NUM_DIRECT_POINTERS {
            if let Some(block_id) = file.direct_pointers[i] {
                if let Some(data) = self.blocks.get(&block_id) {
                   file_data.extend(data); 
                }
            }
        }
       let output = String::from_utf8(file_data).unwrap_or_else(|_| "Error: Invalid UTF-8 data".to_string()); 
       println!("Output: {:?}", output);
       output
    }

    //list all files and directories 
    fn list_directories_and_files(&self) {
        fn is_directory(inode: &Inode, fs: &FileSystem) {
            println!("Directory {} (ID:{:?}): ", inode.file_name, inode.id);
            if let Some(entries) = &inode.entries {
                for &entry_id in entries {
                    if let Some(child_inode) = fs.inodes.get(&entry_id) {
                        match child_inode.inode_type {
                            InodeType::Directory => {
                                is_directory(child_inode, fs);
                            }
                            InodeType::File => {
                                println!(" - File {} (ID: {:?}, Size: {:?} bytes)", child_inode.file_name, child_inode.id, child_inode.file_size);
                            }
                        }
                    }
                }
                
            }
        }
        for (_, inode) in &self.inodes {
            if matches!(inode.inode_type, InodeType::Directory) {
                is_directory(inode, self);
            }
        }
    }

    //undo a function 
    fn undo(&mut self) {
        if let Some(recent) = self.journal.entries.pop(){
            println!("Undid Operation: {}", recent.operation);

            if recent.operation.contains("CREATE DIRECTORY") {
                if let Some(op_name) = recent.details.split("ID: ").nth(1) {
                    let directory_id: u64 = op_name.parse().unwrap();
                    self.inodes.remove(&directory_id);
                    println!(" - Removed Directory: {}", directory_id);
                }
            }else if recent.operation.contains("CREATE FILE") {
                if let Some(op_name) = recent.details.split("ID: ").nth(1) {
                    let file_id: u64 = op_name.parse().unwrap();
                    self.inodes.remove(&file_id);
                    println!(" - Removed File id: {}", file_id);
                }
            }else if recent.operation.contains("ADD FILE"){
                let op_name: Vec<&str> = recent.details.split(", ").collect::<Vec<_>>();
                // println!("{:?}", op_name);
                let file_id: u64 = op_name[0].replace("ID: ", "").parse().expect("Invalid file ID");
                let directory_id: u64 = op_name[1].replace("Parent ID: ", "").parse().expect("Invalid directory");

                if let Some(directory_inode) = self.inodes.get_mut(&directory_id){
                    if matches!(directory_inode.inode_type, InodeType::Directory) {        
                        directory_inode.entries.as_mut().expect("Entries not found").retain(|&id| id != file_id);
                        println!(" - Removed File id: {} from Directory: {}", file_id, directory_id);
                    }
                }
            }
         
        } else {
            println!("No Operations to undo.")
        }

          
    }
}

//Journal struct 
#[allow(dead_code)]
#[derive(Clone, Debug)]
struct Journal {
    entries: Vec<JournalEntry>,
}

//journal entry struct 
#[allow(dead_code)]
#[derive(Clone, Debug)]
struct JournalEntry { 
    operation: String,
    committed: bool,
    details: String,
}

//journal functions 
impl Journal {
    //mew journal 
    fn new() -> Self {
        Journal {
            entries: Vec::new(),
        }
    }

    //add a journal entry 
    fn add_entry(&mut self, operation_type: String, details: String) {
        let new_entry = JournalEntry {
            operation: operation_type.to_string(),
            committed: false,
            details,
        };
        self.entries.push(new_entry); 
    }

    //commit an entry 
    fn commit(&mut self) {
        for entry in &mut self.entries {
            entry.committed = true;
        }
    }

    //print the entire journal
    fn print_journal(&self) {
        let mut count = 1;
        println!("Journal Entries:");
        for entry in &self.entries {
            println!("{}. {} [Committed: {}]", count, entry.operation, entry.committed);
            count += 1;
        }
    }
}

//main function 
fn main() {
    let mut fs = FileSystem::new();
    let mut dir1 = fs.create_directory("Documents");
    let mut dir2 = fs.create_directory("Pictures");

    let mut file1 = fs.create_file("doc1.txt");
    let file2 = fs.create_file("doc2.txt");
    let file3 = fs.create_file("pic1.jpg");

    fs.add_file_to_directory(&mut dir1, &file1);
    fs.inodes.insert(dir1.id, dir1.clone());
    fs.add_file_to_directory(&mut dir1, &file2);
    fs.inodes.insert(dir1.id, dir1);
    fs.add_file_to_directory(&mut dir2, &file3);
    fs.inodes.insert(dir2.id, dir2);

    file1 = fs.write_to_file(file1.clone(), b"Hello, World!");

    println!("\n=== Directory Listing ===");
    fs.list_directories_and_files();

    println!("\n=== Read File ===");
    fs.read_file(&file1);

    println!("\n=== Journal ===");
    //test journal struct
    // let journal = Journal::new();
    // journal.add_entry("Test");
    // println!("{:?}", fs.journal);
    fs.journal.print_journal();

    println!("\n=== Undo Operation ===");
    fs.undo();
    // fs.undo();
    // fs.undo();
    // fs.undo();
    // fs.undo();
    // fs.undo();

    println!("\n=== Final Journal ===");
    fs.journal.print_journal();

    println!("\n=== New Directory Listing ===");
    fs.list_directories_and_files();

    //testing cases 
    // println!("{:?}", fs.blocks);
    // println!("{:?}", file1.direct_pointers);
    // println!("{:?}", file1);
}
