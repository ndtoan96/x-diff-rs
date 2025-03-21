# x-diff-rs

A library to compare XML files unorderedly.

This library implements the X-Diff algorithm from paper
[X-Diff: An Effective Change Detection Algorithm for XML Documents](https://pages.cs.wisc.edu/~yuanwang/papers/xdiff.pdf).

## Example

```rust
use x_diff_rs::{
    diff,
    tree::{XTree, XTreePrintOptions},
};

fn main() {
    let text1 = r#"
<Profile>
 <Customer>
  <PersonName NameType="Default">
   <NameTitle>Mr.</NameTitle>
   <GivenName>George</GivenName>
   <MiddleName>A.</MiddleName>
   <SurName>Smith</SurName>
  </PersonName>
  <TelephoneInfo PhoneTech="Voice" PhoneUse="Work" >
   <Telephone> <AreaCityCode>206</AreaCityCode>
	<PhoneNumber>813-8698</PhoneNumber>
   </Telephone>
  </TelephoneInfo>
  <PaymentForm>
   ...
  </PaymentForm>
  <Address>
   <StreetNmbr POBox="4321-01">From hell</StreetNmbr>
   <BldgRoom>Suite 800</BldgRoom>
   <CityName>Seattle</CityName>
   <StateProv PostalCode="98108">WA</StateProv>
   <CountryName>USA</CountryName>
  </Address>
  <Address>
   <StreetNmbr POBox="4321-01">1200 Yakima St</StreetNmbr>
   <BldgRoom>Suite 800</BldgRoom>
   <CityName>Seattle</CityName>
   <StateProv PostalCode="98108">WA</StateProv>
   <CountryName>USA</CountryName>
  </Address>
 </Customer>
</Profile>
    "#;

    let text2 = r#"
<Profile>
 <Customer>
  <PersonName NameType="Default">
   <NameTitle>Mr.</NameTitle>
   <GivenName>George</GivenName>
   <MiddleName>A.</MiddleName>
   <SurName>Smith</SurName>
  </PersonName>
  <TelephoneInfo PhoneTech="Voice" PhoneUse="Work" >
   <Telephone> <AreaCityCode>206</AreaCityCode>
	<PhoneNumber>813-8698</PhoneNumber>
   </Telephone>
  </TelephoneInfo>
  <Address>
   <StreetNmbr POBox="4321-01">From hell</StreetNmbr>
   <BldgRoom>Suite 800</BldgRoom>
   <CityName>Seattle</CityName>
   <StateProv PostalCode="98108">WA</StateProv>
   <CountryName>USA</CountryName>
  </Address>
  <Address>
   <StreetNmbr POBox="1234-01">1200 Yakima St</StreetNmbr>
   <BldgRoom>Suite 800</BldgRoom>
   <CityName>Paris</CityName>
   <StateProv PostalCode="98108">WA</StateProv>
   <CountryName>USA</CountryName>
  </Address>
  <Status>Single</Status>
 </Customer>
</Profile>
    "#;
    let tree1 = XTree::parse(&text1).unwrap();
    let tree2 = XTree::parse(&text2).unwrap();
    tree1.print(XTreePrintOptions::default().with_node_id());
    tree2.print(XTreePrintOptions::default().with_node_id());
    let difference = diff(&tree1, &tree2);
    for d in difference {
        println!("{d}");
    }
}
```
