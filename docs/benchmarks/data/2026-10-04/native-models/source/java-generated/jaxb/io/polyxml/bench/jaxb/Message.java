
package io.polyxml.bench.jaxb;

import jakarta.xml.bind.annotation.XmlAccessType;
import jakarta.xml.bind.annotation.XmlAccessorType;
import jakarta.xml.bind.annotation.XmlElement;
import jakarta.xml.bind.annotation.XmlType;


/**
 * <p>Java class for Message complex type.
 * 
 * <p>The following schema fragment specifies the expected content contained within this class.
 * 
 * <pre>
 * &lt;complexType name="Message"&gt;
 *   &lt;complexContent&gt;
 *     &lt;restriction base="{http://www.w3.org/2001/XMLSchema}anyType"&gt;
 *       &lt;sequence&gt;
 *         &lt;element name="id" type="{http://www.w3.org/2001/XMLSchema}string"/&gt;
 *         &lt;element name="status" type="{http://www.w3.org/2001/XMLSchema}string"/&gt;
 *         &lt;element name="timestamp" type="{http://www.w3.org/2001/XMLSchema}string"/&gt;
 *         &lt;element name="amount" type="{http://www.w3.org/2001/XMLSchema}string"/&gt;
 *         &lt;element name="latitude" type="{http://www.w3.org/2001/XMLSchema}string"/&gt;
 *         &lt;element name="longitude" type="{http://www.w3.org/2001/XMLSchema}string"/&gt;
 *         &lt;element name="payload" type="{http://www.w3.org/2001/XMLSchema}string"/&gt;
 *         &lt;element name="optional0" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional1" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional2" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional3" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional4" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional5" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional6" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional7" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional8" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional9" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional10" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional11" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional12" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional13" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional14" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional15" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional16" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional17" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional18" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional19" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional20" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional21" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional22" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional23" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional24" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional25" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional26" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional27" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional28" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional29" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional30" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional31" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional32" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional33" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional34" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional35" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional36" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional37" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional38" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional39" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional40" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional41" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional42" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional43" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional44" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional45" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional46" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional47" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional48" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional49" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional50" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional51" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional52" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional53" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional54" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional55" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional56" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional57" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional58" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional59" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional60" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional61" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional62" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional63" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional64" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional65" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional66" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional67" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional68" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional69" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional70" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional71" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *         &lt;element name="optional72" type="{http://www.w3.org/2001/XMLSchema}string" minOccurs="0"/&gt;
 *       &lt;/sequence&gt;
 *     &lt;/restriction&gt;
 *   &lt;/complexContent&gt;
 * &lt;/complexType&gt;
 * </pre>
 * 
 * 
 */
@XmlAccessorType(XmlAccessType.FIELD)
@XmlType(name = "Message", propOrder = {
    "id",
    "status",
    "timestamp",
    "amount",
    "latitude",
    "longitude",
    "payload",
    "optional0",
    "optional1",
    "optional2",
    "optional3",
    "optional4",
    "optional5",
    "optional6",
    "optional7",
    "optional8",
    "optional9",
    "optional10",
    "optional11",
    "optional12",
    "optional13",
    "optional14",
    "optional15",
    "optional16",
    "optional17",
    "optional18",
    "optional19",
    "optional20",
    "optional21",
    "optional22",
    "optional23",
    "optional24",
    "optional25",
    "optional26",
    "optional27",
    "optional28",
    "optional29",
    "optional30",
    "optional31",
    "optional32",
    "optional33",
    "optional34",
    "optional35",
    "optional36",
    "optional37",
    "optional38",
    "optional39",
    "optional40",
    "optional41",
    "optional42",
    "optional43",
    "optional44",
    "optional45",
    "optional46",
    "optional47",
    "optional48",
    "optional49",
    "optional50",
    "optional51",
    "optional52",
    "optional53",
    "optional54",
    "optional55",
    "optional56",
    "optional57",
    "optional58",
    "optional59",
    "optional60",
    "optional61",
    "optional62",
    "optional63",
    "optional64",
    "optional65",
    "optional66",
    "optional67",
    "optional68",
    "optional69",
    "optional70",
    "optional71",
    "optional72"
})
public class Message {

    @XmlElement(required = true)
    protected String id;
    @XmlElement(required = true)
    protected String status;
    @XmlElement(required = true)
    protected String timestamp;
    @XmlElement(required = true)
    protected String amount;
    @XmlElement(required = true)
    protected String latitude;
    @XmlElement(required = true)
    protected String longitude;
    @XmlElement(required = true)
    protected String payload;
    protected String optional0;
    protected String optional1;
    protected String optional2;
    protected String optional3;
    protected String optional4;
    protected String optional5;
    protected String optional6;
    protected String optional7;
    protected String optional8;
    protected String optional9;
    protected String optional10;
    protected String optional11;
    protected String optional12;
    protected String optional13;
    protected String optional14;
    protected String optional15;
    protected String optional16;
    protected String optional17;
    protected String optional18;
    protected String optional19;
    protected String optional20;
    protected String optional21;
    protected String optional22;
    protected String optional23;
    protected String optional24;
    protected String optional25;
    protected String optional26;
    protected String optional27;
    protected String optional28;
    protected String optional29;
    protected String optional30;
    protected String optional31;
    protected String optional32;
    protected String optional33;
    protected String optional34;
    protected String optional35;
    protected String optional36;
    protected String optional37;
    protected String optional38;
    protected String optional39;
    protected String optional40;
    protected String optional41;
    protected String optional42;
    protected String optional43;
    protected String optional44;
    protected String optional45;
    protected String optional46;
    protected String optional47;
    protected String optional48;
    protected String optional49;
    protected String optional50;
    protected String optional51;
    protected String optional52;
    protected String optional53;
    protected String optional54;
    protected String optional55;
    protected String optional56;
    protected String optional57;
    protected String optional58;
    protected String optional59;
    protected String optional60;
    protected String optional61;
    protected String optional62;
    protected String optional63;
    protected String optional64;
    protected String optional65;
    protected String optional66;
    protected String optional67;
    protected String optional68;
    protected String optional69;
    protected String optional70;
    protected String optional71;
    protected String optional72;

    /**
     * Gets the value of the id property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getId() {
        return id;
    }

    /**
     * Sets the value of the id property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setId(String value) {
        this.id = value;
    }

    /**
     * Gets the value of the status property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getStatus() {
        return status;
    }

    /**
     * Sets the value of the status property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setStatus(String value) {
        this.status = value;
    }

    /**
     * Gets the value of the timestamp property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getTimestamp() {
        return timestamp;
    }

    /**
     * Sets the value of the timestamp property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setTimestamp(String value) {
        this.timestamp = value;
    }

    /**
     * Gets the value of the amount property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getAmount() {
        return amount;
    }

    /**
     * Sets the value of the amount property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setAmount(String value) {
        this.amount = value;
    }

    /**
     * Gets the value of the latitude property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getLatitude() {
        return latitude;
    }

    /**
     * Sets the value of the latitude property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setLatitude(String value) {
        this.latitude = value;
    }

    /**
     * Gets the value of the longitude property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getLongitude() {
        return longitude;
    }

    /**
     * Sets the value of the longitude property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setLongitude(String value) {
        this.longitude = value;
    }

    /**
     * Gets the value of the payload property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getPayload() {
        return payload;
    }

    /**
     * Sets the value of the payload property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setPayload(String value) {
        this.payload = value;
    }

    /**
     * Gets the value of the optional0 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional0() {
        return optional0;
    }

    /**
     * Sets the value of the optional0 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional0(String value) {
        this.optional0 = value;
    }

    /**
     * Gets the value of the optional1 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional1() {
        return optional1;
    }

    /**
     * Sets the value of the optional1 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional1(String value) {
        this.optional1 = value;
    }

    /**
     * Gets the value of the optional2 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional2() {
        return optional2;
    }

    /**
     * Sets the value of the optional2 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional2(String value) {
        this.optional2 = value;
    }

    /**
     * Gets the value of the optional3 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional3() {
        return optional3;
    }

    /**
     * Sets the value of the optional3 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional3(String value) {
        this.optional3 = value;
    }

    /**
     * Gets the value of the optional4 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional4() {
        return optional4;
    }

    /**
     * Sets the value of the optional4 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional4(String value) {
        this.optional4 = value;
    }

    /**
     * Gets the value of the optional5 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional5() {
        return optional5;
    }

    /**
     * Sets the value of the optional5 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional5(String value) {
        this.optional5 = value;
    }

    /**
     * Gets the value of the optional6 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional6() {
        return optional6;
    }

    /**
     * Sets the value of the optional6 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional6(String value) {
        this.optional6 = value;
    }

    /**
     * Gets the value of the optional7 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional7() {
        return optional7;
    }

    /**
     * Sets the value of the optional7 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional7(String value) {
        this.optional7 = value;
    }

    /**
     * Gets the value of the optional8 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional8() {
        return optional8;
    }

    /**
     * Sets the value of the optional8 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional8(String value) {
        this.optional8 = value;
    }

    /**
     * Gets the value of the optional9 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional9() {
        return optional9;
    }

    /**
     * Sets the value of the optional9 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional9(String value) {
        this.optional9 = value;
    }

    /**
     * Gets the value of the optional10 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional10() {
        return optional10;
    }

    /**
     * Sets the value of the optional10 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional10(String value) {
        this.optional10 = value;
    }

    /**
     * Gets the value of the optional11 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional11() {
        return optional11;
    }

    /**
     * Sets the value of the optional11 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional11(String value) {
        this.optional11 = value;
    }

    /**
     * Gets the value of the optional12 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional12() {
        return optional12;
    }

    /**
     * Sets the value of the optional12 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional12(String value) {
        this.optional12 = value;
    }

    /**
     * Gets the value of the optional13 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional13() {
        return optional13;
    }

    /**
     * Sets the value of the optional13 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional13(String value) {
        this.optional13 = value;
    }

    /**
     * Gets the value of the optional14 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional14() {
        return optional14;
    }

    /**
     * Sets the value of the optional14 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional14(String value) {
        this.optional14 = value;
    }

    /**
     * Gets the value of the optional15 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional15() {
        return optional15;
    }

    /**
     * Sets the value of the optional15 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional15(String value) {
        this.optional15 = value;
    }

    /**
     * Gets the value of the optional16 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional16() {
        return optional16;
    }

    /**
     * Sets the value of the optional16 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional16(String value) {
        this.optional16 = value;
    }

    /**
     * Gets the value of the optional17 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional17() {
        return optional17;
    }

    /**
     * Sets the value of the optional17 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional17(String value) {
        this.optional17 = value;
    }

    /**
     * Gets the value of the optional18 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional18() {
        return optional18;
    }

    /**
     * Sets the value of the optional18 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional18(String value) {
        this.optional18 = value;
    }

    /**
     * Gets the value of the optional19 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional19() {
        return optional19;
    }

    /**
     * Sets the value of the optional19 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional19(String value) {
        this.optional19 = value;
    }

    /**
     * Gets the value of the optional20 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional20() {
        return optional20;
    }

    /**
     * Sets the value of the optional20 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional20(String value) {
        this.optional20 = value;
    }

    /**
     * Gets the value of the optional21 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional21() {
        return optional21;
    }

    /**
     * Sets the value of the optional21 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional21(String value) {
        this.optional21 = value;
    }

    /**
     * Gets the value of the optional22 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional22() {
        return optional22;
    }

    /**
     * Sets the value of the optional22 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional22(String value) {
        this.optional22 = value;
    }

    /**
     * Gets the value of the optional23 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional23() {
        return optional23;
    }

    /**
     * Sets the value of the optional23 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional23(String value) {
        this.optional23 = value;
    }

    /**
     * Gets the value of the optional24 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional24() {
        return optional24;
    }

    /**
     * Sets the value of the optional24 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional24(String value) {
        this.optional24 = value;
    }

    /**
     * Gets the value of the optional25 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional25() {
        return optional25;
    }

    /**
     * Sets the value of the optional25 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional25(String value) {
        this.optional25 = value;
    }

    /**
     * Gets the value of the optional26 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional26() {
        return optional26;
    }

    /**
     * Sets the value of the optional26 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional26(String value) {
        this.optional26 = value;
    }

    /**
     * Gets the value of the optional27 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional27() {
        return optional27;
    }

    /**
     * Sets the value of the optional27 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional27(String value) {
        this.optional27 = value;
    }

    /**
     * Gets the value of the optional28 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional28() {
        return optional28;
    }

    /**
     * Sets the value of the optional28 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional28(String value) {
        this.optional28 = value;
    }

    /**
     * Gets the value of the optional29 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional29() {
        return optional29;
    }

    /**
     * Sets the value of the optional29 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional29(String value) {
        this.optional29 = value;
    }

    /**
     * Gets the value of the optional30 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional30() {
        return optional30;
    }

    /**
     * Sets the value of the optional30 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional30(String value) {
        this.optional30 = value;
    }

    /**
     * Gets the value of the optional31 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional31() {
        return optional31;
    }

    /**
     * Sets the value of the optional31 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional31(String value) {
        this.optional31 = value;
    }

    /**
     * Gets the value of the optional32 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional32() {
        return optional32;
    }

    /**
     * Sets the value of the optional32 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional32(String value) {
        this.optional32 = value;
    }

    /**
     * Gets the value of the optional33 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional33() {
        return optional33;
    }

    /**
     * Sets the value of the optional33 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional33(String value) {
        this.optional33 = value;
    }

    /**
     * Gets the value of the optional34 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional34() {
        return optional34;
    }

    /**
     * Sets the value of the optional34 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional34(String value) {
        this.optional34 = value;
    }

    /**
     * Gets the value of the optional35 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional35() {
        return optional35;
    }

    /**
     * Sets the value of the optional35 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional35(String value) {
        this.optional35 = value;
    }

    /**
     * Gets the value of the optional36 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional36() {
        return optional36;
    }

    /**
     * Sets the value of the optional36 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional36(String value) {
        this.optional36 = value;
    }

    /**
     * Gets the value of the optional37 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional37() {
        return optional37;
    }

    /**
     * Sets the value of the optional37 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional37(String value) {
        this.optional37 = value;
    }

    /**
     * Gets the value of the optional38 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional38() {
        return optional38;
    }

    /**
     * Sets the value of the optional38 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional38(String value) {
        this.optional38 = value;
    }

    /**
     * Gets the value of the optional39 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional39() {
        return optional39;
    }

    /**
     * Sets the value of the optional39 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional39(String value) {
        this.optional39 = value;
    }

    /**
     * Gets the value of the optional40 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional40() {
        return optional40;
    }

    /**
     * Sets the value of the optional40 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional40(String value) {
        this.optional40 = value;
    }

    /**
     * Gets the value of the optional41 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional41() {
        return optional41;
    }

    /**
     * Sets the value of the optional41 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional41(String value) {
        this.optional41 = value;
    }

    /**
     * Gets the value of the optional42 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional42() {
        return optional42;
    }

    /**
     * Sets the value of the optional42 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional42(String value) {
        this.optional42 = value;
    }

    /**
     * Gets the value of the optional43 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional43() {
        return optional43;
    }

    /**
     * Sets the value of the optional43 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional43(String value) {
        this.optional43 = value;
    }

    /**
     * Gets the value of the optional44 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional44() {
        return optional44;
    }

    /**
     * Sets the value of the optional44 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional44(String value) {
        this.optional44 = value;
    }

    /**
     * Gets the value of the optional45 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional45() {
        return optional45;
    }

    /**
     * Sets the value of the optional45 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional45(String value) {
        this.optional45 = value;
    }

    /**
     * Gets the value of the optional46 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional46() {
        return optional46;
    }

    /**
     * Sets the value of the optional46 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional46(String value) {
        this.optional46 = value;
    }

    /**
     * Gets the value of the optional47 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional47() {
        return optional47;
    }

    /**
     * Sets the value of the optional47 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional47(String value) {
        this.optional47 = value;
    }

    /**
     * Gets the value of the optional48 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional48() {
        return optional48;
    }

    /**
     * Sets the value of the optional48 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional48(String value) {
        this.optional48 = value;
    }

    /**
     * Gets the value of the optional49 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional49() {
        return optional49;
    }

    /**
     * Sets the value of the optional49 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional49(String value) {
        this.optional49 = value;
    }

    /**
     * Gets the value of the optional50 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional50() {
        return optional50;
    }

    /**
     * Sets the value of the optional50 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional50(String value) {
        this.optional50 = value;
    }

    /**
     * Gets the value of the optional51 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional51() {
        return optional51;
    }

    /**
     * Sets the value of the optional51 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional51(String value) {
        this.optional51 = value;
    }

    /**
     * Gets the value of the optional52 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional52() {
        return optional52;
    }

    /**
     * Sets the value of the optional52 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional52(String value) {
        this.optional52 = value;
    }

    /**
     * Gets the value of the optional53 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional53() {
        return optional53;
    }

    /**
     * Sets the value of the optional53 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional53(String value) {
        this.optional53 = value;
    }

    /**
     * Gets the value of the optional54 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional54() {
        return optional54;
    }

    /**
     * Sets the value of the optional54 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional54(String value) {
        this.optional54 = value;
    }

    /**
     * Gets the value of the optional55 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional55() {
        return optional55;
    }

    /**
     * Sets the value of the optional55 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional55(String value) {
        this.optional55 = value;
    }

    /**
     * Gets the value of the optional56 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional56() {
        return optional56;
    }

    /**
     * Sets the value of the optional56 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional56(String value) {
        this.optional56 = value;
    }

    /**
     * Gets the value of the optional57 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional57() {
        return optional57;
    }

    /**
     * Sets the value of the optional57 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional57(String value) {
        this.optional57 = value;
    }

    /**
     * Gets the value of the optional58 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional58() {
        return optional58;
    }

    /**
     * Sets the value of the optional58 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional58(String value) {
        this.optional58 = value;
    }

    /**
     * Gets the value of the optional59 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional59() {
        return optional59;
    }

    /**
     * Sets the value of the optional59 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional59(String value) {
        this.optional59 = value;
    }

    /**
     * Gets the value of the optional60 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional60() {
        return optional60;
    }

    /**
     * Sets the value of the optional60 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional60(String value) {
        this.optional60 = value;
    }

    /**
     * Gets the value of the optional61 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional61() {
        return optional61;
    }

    /**
     * Sets the value of the optional61 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional61(String value) {
        this.optional61 = value;
    }

    /**
     * Gets the value of the optional62 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional62() {
        return optional62;
    }

    /**
     * Sets the value of the optional62 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional62(String value) {
        this.optional62 = value;
    }

    /**
     * Gets the value of the optional63 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional63() {
        return optional63;
    }

    /**
     * Sets the value of the optional63 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional63(String value) {
        this.optional63 = value;
    }

    /**
     * Gets the value of the optional64 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional64() {
        return optional64;
    }

    /**
     * Sets the value of the optional64 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional64(String value) {
        this.optional64 = value;
    }

    /**
     * Gets the value of the optional65 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional65() {
        return optional65;
    }

    /**
     * Sets the value of the optional65 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional65(String value) {
        this.optional65 = value;
    }

    /**
     * Gets the value of the optional66 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional66() {
        return optional66;
    }

    /**
     * Sets the value of the optional66 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional66(String value) {
        this.optional66 = value;
    }

    /**
     * Gets the value of the optional67 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional67() {
        return optional67;
    }

    /**
     * Sets the value of the optional67 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional67(String value) {
        this.optional67 = value;
    }

    /**
     * Gets the value of the optional68 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional68() {
        return optional68;
    }

    /**
     * Sets the value of the optional68 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional68(String value) {
        this.optional68 = value;
    }

    /**
     * Gets the value of the optional69 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional69() {
        return optional69;
    }

    /**
     * Sets the value of the optional69 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional69(String value) {
        this.optional69 = value;
    }

    /**
     * Gets the value of the optional70 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional70() {
        return optional70;
    }

    /**
     * Sets the value of the optional70 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional70(String value) {
        this.optional70 = value;
    }

    /**
     * Gets the value of the optional71 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional71() {
        return optional71;
    }

    /**
     * Sets the value of the optional71 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional71(String value) {
        this.optional71 = value;
    }

    /**
     * Gets the value of the optional72 property.
     * 
     * @return
     *     possible object is
     *     {@link String }
     *     
     */
    public String getOptional72() {
        return optional72;
    }

    /**
     * Sets the value of the optional72 property.
     * 
     * @param value
     *     allowed object is
     *     {@link String }
     *     
     */
    public void setOptional72(String value) {
        this.optional72 = value;
    }

}
